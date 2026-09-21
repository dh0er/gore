        TArray<AGothicCharacter> local_8 = this.GatherSensedFighters();
        if (IsValid(this.WatchGroup) && ((local_8.Num() > 0)))
        {
            this.WatchGroup.UpdateAnchorAndAxis(local_8);
            this.WatchGroup.IdealRadius = this.IdealRadius;
            this.LastFightAnchor = this.WatchGroup.AnchorLocation;
            this.LastFightAxis = this.WatchGroup.FightAxisXY;
        }
        else
        {
            if (local_8.Num() > 0)
            {
                FVector local_22 = FVector(FVector::ZeroVector);
                for (auto local_36 : local_8)
                {
                    local_22 += local_36.GetActorLocation();
                }
                this.LastFightAnchor = (local_22 / float(Math::Max(1, local_8.Num())));
            }
        }
        if (local_8.Num() >= 1)
        {
            this.bObservedMultipleFightParticipants = this.bObservedMultipleFightParticipants || ((local_8.Num() >= 2));
        }
        if (this.bEnteredWithoutSensedFighter && !(this.bObservedMultipleFightParticipants))
        {
            float local_52 = (this.ActivationTime.IsValid() ? this.ActivationTime.GetAgeInRealtimeSeconds() : 0.0f);
            if (local_52 >= this.HearingApproachTimeoutSeconds)
            {
                this.LastRepositionReason = "hearing-timeout";
                this.HandleFightEnded();
                return;
            }
            if (local_8.Num() == 0 && IsValid(this.RememberedPerception.Origin.GetCharacter()))
            {
                this.LastRepositionReason = "approach-perception-origin";
                this.SetWalkSpeed(EWalkSpeed(1));
                ::GotoPosition(this.AI, this.RememberedPerception.Origin.GetCharacter().GetActorLocation(), this.DistanceSoftMax, -1.0);
                return;
            }
        }
        bool local_12 = !(this.bEnteredWithoutSensedFighter) || this.bObservedMultipleFightParticipants;
        if (local_8.Num() < 2 && local_12)
        {
            this.HandleFightEnded();
            return;
        }
        if (local_8.Num() == 0)
        {
            this.LastRepositionReason = "hold-no-fighters";
            return;
        }
        AGothicCharacter local_36_2 = this.SelectClosestFighter(local_8);
        if (IsValid(local_36_2))
        {
            this.AI.TargetCharacter(local_36_2, this.FocusPriority, this);
        }
        if (::IsSitting(this.GetSelf()))
        {
            return;
        }
        if (this.bDisableSpread)
        {
            this.LastRepositionReason = "spread-disabled";
            return;
        }
        int local_11 = (IsValid(this.WatchGroup) ? this.WatchGroup.GetMemberCount() : 1);
        bool local_9 = (local_11 >= 3) || this.bForceSlotMode;
        if (this.bForceBoidsMode)
        {
            local_9 = false;
        }
        this.bLastWasSlotMode = local_9;
        FVector local_22_2 = (IsValid(this.WatchGroup) && !(this.WatchGroup.AnchorLocation.IsNearlyZero(9.999999747378752e-5)) ? this.WatchGroup.AnchorLocation : this.LastFightAnchor);
        this.LastAnchorRef = local_22_2;
        this.RefreshPreferredSide(local_22_2);
        this.RefreshLoSBypassIfAnchorIndoors(local_22_2);
        this.InvalidateSettleIfToggled();
        if (local_11 != this.LastWatcherCount)
        {
            this.bSettled = false;
            this.LastWatcherCount = local_11;
        }
        if (this.bSettled)
        {
            if (!(this.ShouldLeaveSettle(local_22_2, local_9)))
            {
                this.LastRepositionReason = "settled";
                this.bLastArcDetour = false;
                return;
            }
            this.bSettled = false;
        }
        FVector local_78(FVector::ZeroVector);
        bool local_79 = false;
        if (local_9 && IsValid(this.WatchGroup) && !(this.WatchGroup.AnchorLocation.IsNearlyZero(9.999999747378752e-5)))
        {
            float local_16 = this.WatchGroup.GetSlotAngleForMember(this.GetSelf());
            float local_82 = this.AngularRetryStepDeg * 0.017453292f;
            int local_10_3 = Math::Max(0, this.MaxAngularRetriesPerSide);
            int local_49_4 = (2 * local_10_3) + 1;
            int local_85 = 0;
            for (; local_85 < local_49_4 && !local_79; ++local_85)
            {
                int local_89 = (local_85 == 0 ? 0 : (local_85 % 2 == 1 ? Math::IntegerDivisionTrunc(local_85 + 1, 2) : -Math::IntegerDivisionTrunc(local_85, 2)));
                float local_60 = local_16 + (local_89 * local_82);
                FVector local_42 = (local_22_2 + (FVector(Math::Cos(local_60), Math::Sin(local_60), 0.0) * this.IdealRadius));
                local_42.Z = local_22_2.Z;
                if (!(this.IsCandidateOnPreferredSide(local_42, local_22_2)))
                {
                    DebugScript::DrawSphere((local_42 + (FVector(FVector::UpVector) * 20.0)), 12.0f, FColor::Orange, 0.3f);
                }
                else
                {
                    FVector local_106;
                    if (this.ValidateNavCandidate(local_42, local_22_2, local_106))
                    {
                        local_78 = local_106;
                        this.LastRepositionReason = FString().Append("slot[").Append(local_89).Append("]");
                        local_79 = true;
                        break;
                    }
                    DebugScript::DrawSphere((local_42 + (FVector(FVector::UpVector) * 20.0)), 15.0f, FColor::Red, 0.3f);
                }
            }
            if (!(local_79))
            {
                this.LastRepositionReason = "slot-all-rejected-hold";
                this.AI.GetAIController().StopMovement();
                this.bHasIssuedMoveTarget = false;
                return;
            }
        }
        else
        {
            FVector local_98_2 = this.ComputeBoidsTarget(local_8, this.LastFightAnchor);
            local_98_2.Z = local_22_2.Z;
            if (this.bRequireSideLock && (this.PreferredSideSign != 0) && !(this.IsCandidateOnPreferredSide(local_98_2, local_22_2)))
            {
                local_98_2 = this.ClampToPreferredSide(local_98_2, local_22_2);
            }
            FVector local_42;
            if (!(this.ValidateNavCandidate(local_98_2, local_22_2, local_42)) || !(this.IsCandidateOnPreferredSide(local_42, local_22_2)))
            {
                this.LastRepositionReason = "boids-rejected-hold";
                DebugScript::DrawSphere((local_98_2 + (FVector(FVector::UpVector) * 20.0)), 15.0f, FColor::Red, 0.3f);
                this.AI.GetAIController().StopMovement();
                this.bHasIssuedMoveTarget = false;
                return;
            }
            local_78 = local_42;
            this.LastRepositionReason = "boids";
        }
        if (IsValid(this.WatchGroup) && this.SegmentIntersectsFightZone(this.GetSelf().GetFeetLocation(), local_78, this.WatchGroup.AnchorLocation, this.WatchGroup.FightAxisXY, this.WatchGroup.FightAxisHalfLength, this.FighterAvoidanceLateralBand))
        {
            FVector local_106_2 = this.ComputeDetourWaypoint(this.GetSelf().GetFeetLocation(), local_78, this.WatchGroup.AnchorLocation, this.WatchGroup.FightAxisXY, this.WatchGroup.FightAxisHalfLength, this.FighterAvoidanceLateralBand);
            FVector local_98_3;
            if (this.ValidateNavCandidate(local_106_2, local_22_2, local_98_3))
            {
                local_78 = local_98_3;
                this.LastRepositionReason = (FString(this.LastRepositionReason) + "+detour");
            }
            else
            {
                this.LastRepositionReason = (FString(this.LastRepositionReason) + "+detour-rejected-hold");
                DebugScript::DrawSphere((local_106_2 + (FVector(FVector::UpVector) * 20.0)), 15.0f, FColor::Red, 0.3f);
                this.AI.GetAIController().StopMovement();
                this.bHasIssuedMoveTarget = false;
                return;
            }
        }
        this.bLastArcDetour = false;
        float local_82_2 = this.IdealRadius * this.InnerForbiddenFactor;
        FVector local_106_3 = FVector(local_22_2);
        local_106_3.Z = 0.0;
        FVector local_98_4 = FVector(this.GetSelf().GetFeetLocation());
        local_98_4.Z = 0.0;
        bool local_69 = (local_98_4 - local_106_3).Size() > local_82_2;
        if (local_69 && this.SegmentEntersInnerRing(this.GetSelf().GetFeetLocation(), local_78, local_22_2, local_82_2))
        {
            FVector local_42 = this.ComputeArcTangentWaypoint(this.GetSelf().GetFeetLocation(), local_78, local_22_2, this.IdealRadius, this.ArcStepDeg);
            this.LastArcWaypoint = local_42;
            FVector local_130;
            if (this.IsCandidateOnPreferredSide(local_42, local_22_2) && this.ValidateNavCandidate(local_42, local_22_2, local_130) && this.IsCandidateOnPreferredSide(local_130, local_22_2) && !(this.SegmentEntersInnerRing(this.GetSelf().GetFeetLocation(), local_130, local_22_2, local_82_2)))
            {
                local_78 = local_130;
                this.LastRepositionReason = (FString(this.LastRepositionReason) + "+arc");
                this.bLastArcDetour = true;
            }
            else
            {
                this.LastRepositionReason = (FString(this.LastRepositionReason) + "+arc-rejected-hold");
                DebugScript::DrawSphere((local_42 + (FVector(FVector::UpVector) * 20.0)), 15.0f, FColor::Red, 0.3f);
                this.AI.GetAIController().StopMovement();
                this.bHasIssuedMoveTarget = false;
                return;
            }
        }
        if (this.ShouldEnterSettle(local_22_2, local_9))
        {
            this.bSettled = true;
            this.SettledAtAnchor = local_22_2;
            float local_16_2 = 0.0;
            if (this.ComputeSlotAngleErrorRad(local_22_2, local_16_2))
            {
                this.SettledAtAngleRad = local_16_2;
            }
            this.LastRepositionReason = (FString(this.LastRepositionReason) + "+enter-settle");
            this.AI.GetAIController().StopMovement();
            this.bHasIssuedMoveTarget = false;
            return;
        }
        this.LastTargetSpot = local_78;
        float local_100_2 = this.GetSelf().GetFeetLocation().Distance(local_78);
        if (local_100_2 <= this.ArrivalDistance)
        {
            this.AI.GetAIController().StopMovement();
            this.bHasIssuedMoveTarget = false;
            return;
        }
        float local_16_3 = 1.7976931348623157e308;
        if (IsValid(local_36_2))
        {
            local_16_3 = this.GetSelf().GetFeetLocation().Distance(local_36_2.GetFeetLocation());
        }
        bool local_72 = local_16_3 < this.DistanceTooClose;
        EWalkSpeed local_57;
        if (local_72 || ((local_16_3 > this.DistanceTooFar)) || (local_100_2 > 400.0))
        {
            local_57 = EWalkSpeed(1);
        }
        else
        {
            local_57 = EWalkSpeed(0);
        }
        this.SetWalkSpeed(EWalkSpeed(local_57));
        if (this.ShouldKeepCurrentMoveOrder(local_78))
        {
            this.LastRepositionReason = (FString(this.LastRepositionReason) + "+keep-move");
            return;
        }
        ::GotoPosition(this.AI, local_78, this.ArrivalDistance, -1.0);
        this.LastIssuedMoveTarget = local_78;
        this.LastMoveOrderAt = FInGameTime::Now();
        this.bHasIssuedMoveTarget = true;
        return;
