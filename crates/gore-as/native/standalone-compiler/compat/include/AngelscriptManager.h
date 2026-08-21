#pragma once

#include "AngelscriptSettings.h"

class FAngelscriptManager final {
public:
    static FAngelscriptManager& Get() {
        static FAngelscriptManager instance;
        return instance;
    }

    inline static bool bSimulateCooked = false;
    UAngelscriptSettings* ConfigSettings = &settings_;

private:
    UAngelscriptSettings settings_{};
};
