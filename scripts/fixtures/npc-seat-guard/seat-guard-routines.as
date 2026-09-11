// Daytime use of an unrestricted chair and nearby night-watch point.
class UDailyRoutine_GoreSeatGuardActivities : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Sit(), n"IO_OC_CHAIR_81", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(12, 0, UAIState_GuardWatch(), n"BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(18, 0, UAIState_Sit(), n"IO_OC_CHAIR_81", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreSeatGuardObserver : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Stand(), n"WP_OC_NorthGate", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}
