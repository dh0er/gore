#pragma once

#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/precompiled_data.hpp"

#include <string>

namespace gore::as::standalone::precompiled {

struct metadata_projection_result {
    bool ok = true;
    std::string detail;
};

// Applies the source-only FAngelscriptModuleDesc fields that cannot be
// recovered from asCModule. The update is atomic and rejects any reflected
// class/property/function descriptor that does not map 1:1 to the compiled
// engine record. ComposeOnto is copied only when an external ClassAnalyze hook
// supplied it; this layer never invents that game-specific result.
metadata_projection_result project_preprocessed_metadata(
    const lexical_module_description& description,
    precompiled_module& module);

// Restores descriptor-only fields from an unchanged cached module onto a
// freshly exported engine record. Positional identity is accepted only after
// all class/property/function names and counts match exactly.
metadata_projection_result preserve_cached_metadata(
    const precompiled_module& cached,
    precompiled_module& rebuilt);

} // namespace gore::as::standalone::precompiled
