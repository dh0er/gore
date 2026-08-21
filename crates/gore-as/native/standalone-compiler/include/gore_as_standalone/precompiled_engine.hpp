#pragma once

#include "gore_as_standalone/precompiled_data.hpp"

#include "angelscript.h"

#include <cstddef>
#include <string>
#include <vector>

namespace gore::as::standalone::precompiled {

enum class engine_bridge_phase {
    none,
    preflight,
    request_build,
    prepare_engine,
    create_modules,
    create_types,
    create_globals_and_functions,
    relocate_bytecode,
    initialize_globals,
    export_module,
    cleanup,
};

struct engine_bridge_result {
    int code = asSUCCESS;
    engine_bridge_phase phase = engine_bridge_phase::none;
    std::size_t module_index = static_cast<std::size_t>(-1);
    std::string detail;

    [[nodiscard]] bool succeeded() const noexcept { return code >= 0; }
};

// Current engine bridge checkpoint. It restores the generic fork-side portion
// of the three PrecompiledData apply stages: module imports/function-import
// declarations, script enums/classes/inheritance/layout, methods/constructors/
// destructors/behaviours, globals and initializers, plus type/function/global/
// property bytecode references. The complete cache is preflighted before the
// engine is mutated. Unreal shadow layouts and metadata, delegate/event tags,
// string-literal globals, statics/post-init hooks and profile registry replay
// remain intentionally fail-closed.
engine_bridge_result rehydrate_cache_checkpoint(
    asIScriptEngine& engine,
    const cache& input,
    std::vector<asIScriptModule*>& modules);

// Export one compiled module through the same generic fork-side subset.
// `module_key` is the outer UE FString key; `script_relative_filename`,
// `code_hash`, direct import ordering and the Unreal/preprocessor-only fields
// are authoritative preprocessing inputs absent from the generic engine
// object. Passing `reference_tables` enables reference-bearing records and
// stages all tail-table changes atomically with `output`.
engine_bridge_result export_module_checkpoint(
    asIScriptModule& module,
    const map_string& module_key,
    const archive_string& script_relative_filename,
    std::int64_t code_hash,
    precompiled_module& output,
    cache* reference_tables = nullptr);

} // namespace gore::as::standalone::precompiled
