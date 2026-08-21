#pragma once

#include "angelscript.h"

namespace gore::as::standalone {

// Single-module build-only adapter for the generic core smoke. UNREANGEL's
// FAngelscriptManager applies phase barriers over the complete module graph;
// invoking every phase sequentially for one module is not manager parity and
// must not become the production multi-module orchestrator.
int build_module(asIScriptModule& module);

} // namespace gore::as::standalone
