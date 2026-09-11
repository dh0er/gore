// gore-as decompiled module: LevelScripts.XardasTower_AI (LevelScripts/XardasTower_AI.as)
// NOTE: local names + string literals are not stored in the cache.


class UOW_XT_DEMON_LESSER_SPAWN_WP : UWorldPointScript
{
    UOW_XT_DEMON_LESSER_SPAWN_WP()
    {
        return;
    }
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_XT_XardasDemon::StaticClass()), nullptr);
        return;
    }
}

class UWP_WarningFlock_XT_01 : UWorldPointScript
{
    UWP_WarningFlock_XT_01()
    {
        return;
    }
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_GORE_TEST_A::StaticClass()), UDailyRoutine_GORE_TEST_A_Start());
        this.SpawnWarningFlock(TSubclassOf<UAmbientLifeConfig>(UCrowWarningAreaMid_AmbientLife_Config::StaticClass()), 4, 80.0f, 7.0f);
        return;
    }
}

class UWP_WarningFlock_XT_02 : UWorldPointScript
{
    UWP_WarningFlock_XT_02()
    {
        return;
    }
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_GORE_TEST_B::StaticClass()), UDailyRoutine_GORE_TEST_B_Start());
        this.SpawnWarningFlock(TSubclassOf<UAmbientLifeConfig>(UCrowWarningArea_AmbientLife_Config::StaticClass()), 3, 150.0f, 2.0f);
        return;
    }
}

class UXT_Skeleton_SPAWN_WP : UWorldPointScript
{
    UXT_Skeleton_SPAWN_WP()
    {
        return;
    }
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Skeleton_XardasServant::StaticClass()), UDailyRoutine_Skeleton_Servant());
        return;
    }
}

