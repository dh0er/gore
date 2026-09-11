// No speech requests: stock Human perception and UGA_Human_Mumble provide voice.
// TestGotoWP uses these named points only for navigation, avoiding ambient-spot
// unique-name restrictions. The Human daily routine installs neutral greetings.
class UDailyRoutine_GoreNaturalVoiceB : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_TestGotoWP(), n"FP_NavigationSupport393", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(14, 0, UAIState_TestGotoWP(), n"FP_XT_WAIT_OUTSIDE", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(16, 0, UAIState_TestGotoWP(), n"FP_NavigationSupport393", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreNaturalVoiceC : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_TestGotoWP(), n"FP_NavigationSupport391", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreNaturalVoiceObserver : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_TestGotoWP(), n"FP_NavigationSupport392", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}
