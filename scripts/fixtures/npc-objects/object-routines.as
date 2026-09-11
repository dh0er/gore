// Exact bed/alchemy pairings from Xardas' shipped bedroom routine.
// Only initial setup teleports B; scheduled phase changes use normal navigation.
class UDailyRoutine_GoreObjectActivities : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Sleep(), n"Interactive_Bed_Xardas472597", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(12, 0, UAIState_PotionAlchemy(), n"IO_XT_POTION_ALCHEMY_01", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(18, 0, UAIState_Sleep(), n"Interactive_Bed_Xardas472597", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreObjectObserver : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    // Navigation-only use; do not request the Xardas-restricted StandAround action.
    default Schedule(0, 0, UAIState_Stand(), n"FP_XT_STANDAROUND_01", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}
