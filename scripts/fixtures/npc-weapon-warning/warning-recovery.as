// B-only recovery for a completed threat assessment that leaves an empty
// warning state running. Keep the shipped warning, voice and escalation paths.
class UAIState_GoreBWarningRecovery : UAIState_Warning_Crime_Human_WeaponDrawn
{
    UPROPERTY()
    bool RecoveryRecorded = false;

    void BeginWarning()
    {
        Super::BeginWarning();
        AG1RGameState::SaveWorldFloatData(this.GetCharacterState().GetWorld(), n"gore_b_warning_fix_v1_active", 1.0f);
    }

    void DoInLoopIfCharacterNotStillWarning()
    {
        if (this.bShouldExitState) return;
        if (this.bEndAssessmentDone
            && this.GetCharacterOfInterest() == nullptr
            && this.GetCharactersBeingWarned().IsEmpty()
            && this.AI.GetSensedLivingEnemies(false).IsEmpty())
        {
            if (!this.RecoveryRecorded)
            {
                this.RecoveryRecorded = true;
                AG1RGameState::SaveWorldFloatData(this.GetCharacterState().GetWorld(), n"gore_b_warning_fix_v1_recovered", 1.0f);
            }
            // Use the regular crime assessment and its normal conflict cleanup.
            // The diagnostic flag must not prevent a retry if assessment waits.
            this.AssessEndWarning();
            return;
        }
        Super::DoInLoopIfCharacterNotStillWarning();
    }
}

// Diego adds only this scoring default to Human. Derive from the same supported
// parent because the shipped Diego leaf has a final defaults initializer.
class UGameplayAbility_CharacterAI_GoreBWarningRecovery : UGameplayAbility_CharacterAI_Human
{
    default AddWeightedCombatTargetScoringEntry(UAICombatTargetScoringEntry_ZombieBias(), 10000.0);
    default SetAIStateClassForType(GameplayTag::AIState_Conflict_Warning_Threat, UAIState_GoreBWarningRecovery);
}
