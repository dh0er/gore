// A is the trader and teacher. All stock is seeded explicitly once per test save.
// B, C and every existing character definition remain unchanged.
class UTraderConfig_GoreRoleEconomy : UTraderConfigBase
{
    default m_UniqueName = n"GORE_TEST_A";
    // Existing base/default region and type; stock is deliberately menu-seeded
    // so this fixture also works on an already-started test save.
}

void GoreRoleEconomySnapshot(AGothicCharacterState Teacher, AGothicCharacterState Hero)
{
    if (Teacher == nullptr || Hero == nullptr) return;
    GoreRoleNote(Hero, n"gore_role_hero_ore", float32(::GetOreCount(Hero.GetInventory())));
    GoreRoleNote(Hero, n"gore_role_hero_lp", ::GetSkillpointsAttribute(Hero));
    GoreRoleNote(Hero, n"gore_role_diving_known", ::HasLearnedSkill(Hero, UGE_Skill_Diving) ? 1.0f : 0.0f);
    GoreRoleNote(Hero, n"gore_role_enough_ore", ::HasEnoughOreToLearnSkill(Hero, UGE_Skill_Diving) ? 1.0f : 0.0f);
    GoreRoleNote(Hero, n"gore_role_enough_lp", ::HasEnoughSkillpointsToLearnSkill(Hero, UGE_Skill_Diving) ? 1.0f : 0.0f);
}

namespace G1R::Conversation
{
class UChoiceGoreRoleStock : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146001;
    default Caption = FText::FromString(n"01 Handel: einmalig 3 Kaese, 10 Pfeile, 100 Erz".ToString());
    default PriorityRank = 60;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreRoleRead(this.GetSelf(), n"gore_role_stock_seeded") == 0.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (this.GetSelf() == nullptr || this.GetSelf().GetInventory() == nullptr) return;
        ::AddItemToInventory(this.GetSelf(), UItFo_Cheese, 3, EInventoryTypes::Trader);
        ::AddItemToInventory(this.GetSelf(), UItAm_Arrow, 10, EInventoryTypes::Trader);
        ::AddItemToInventory(this.GetSelf(), UItMi_Orenugget, 100, EInventoryTypes::Trader);
        GoreRoleNote(this.GetSelf(), n"gore_role_stock_seeded", 1.0f);
        GoreRoleEconomySnapshot(this.GetSelf(), Hero());
    }
}

class UChoiceGoreRoleTrade : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146002;
    default Caption = FText::FromString(n"02 Handel: kaufen / verkaufen".ToString());
    default PriorityRank = 59;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreRoleRead(this.GetSelf(), n"gore_role_stock_seeded") == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (this.GetSelf().GetAI() == nullptr || Hero() == nullptr) return;
        GoreRoleEconomySnapshot(this.GetSelf(), Hero());
        ::StartTradingWith(this.GetSelf().GetAI(), Hero());
    }
}

class UChoiceGoreRoleLearn : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146003;
    default Caption = FText::FromString(n"03 Lehrer: Tauchen lernen (5 LP / 30 Erz)".ToString());
    default PriorityRank = 58;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Hero() != nullptr; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Hero() == nullptr || Hero().GetAI() == nullptr) return;
        GoreRoleNote(Hero(), n"gore_role_learn_before_ore", float32(::GetOreCount(Hero().GetInventory())));
        GoreRoleNote(Hero(), n"gore_role_learn_before_lp", ::GetSkillpointsAttribute(Hero()));
        // Stock helper checks points, ore and already learned before applying a skill.
        bool Learned = ::TryLearnSkill(Hero().GetAI(), UGE_Skill_Diving, this.GetSelf());
        GoreRoleNote(Hero(), n"gore_role_learn_result", Learned ? 1.0f : 0.0f);
        GoreRoleEconomySnapshot(this.GetSelf(), Hero());
    }
}

class UChoiceGoreRoleFunds : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146004;
    default Caption = FText::FromString(n"04 Testmittel: einmalig +5 LP und +50 Erz".ToString());
    default PriorityRank = 57;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Hero() != nullptr && GoreRoleRead(Hero(), n"gore_role_funds_given") == 0.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Hero() == nullptr || Hero().AbilitySystemComponent == nullptr) return;
        FGameplayEffectSpecHandle Points = Hero().AbilitySystemComponent.MakeOutgoingSpec(
            TSubclassOf<UGameplayEffect>(UGE_SkillPay_SP::StaticClass()), 1.0f,
            Hero().AbilitySystemComponent.MakeEffectContext());
        Points.GetSpec().SetByCallerMagnitude(GameplayTag::GE_Param_Skillpoints, 5.0f);
        Hero().AbilitySystemComponent.ApplyGameplayEffectSpecToSelf(Points);
        ::AddItemToInventory(Hero(), UItMi_Orenugget, 50);
        GoreRoleNote(Hero(), n"gore_role_funds_given", 1.0f);
        GoreRoleEconomySnapshot(this.GetSelf(), Hero());
    }
}

class UChoiceGoreRoleEconomyRead : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146005;
    default Caption = FText::FromString(n"05 Handel / Lernen: Zustand protokollieren".ToString());
    default PriorityRank = 56;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { GoreRoleEconomySnapshot(this.GetSelf(), Hero()); this.EndConversation(); }
}
}
