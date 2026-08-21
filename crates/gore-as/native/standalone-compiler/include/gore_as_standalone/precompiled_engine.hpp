#pragma once

#include "gore_as_standalone/precompiled_data.hpp"
#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/registry_profile.hpp"

#include "angelscript.h"

#include <cstddef>
#include <string>
#include <vector>

namespace gore::as::standalone {
class frontend_compile_runtime;
}

namespace gore::as::standalone::precompiled {

enum class engine_bridge_phase {
    none,
    preflight,
    request_build,
    prepare_engine,
    create_modules,
    create_types,
    parse_source,
    generate_source_types,
    create_globals_and_functions,
    generate_source_functions,
    layout_types,
    layout_source_functions,
    relocate_bytecode,
    compile_source_code,
    validate_template_instances,
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

// Rebuild a final graph from a pristine cache plus source overlays under one
// RequestBuild/BuildCompleted session. A source module whose name is present
// in `base` replaces that cached module; any other source module is appended.
// Saved references are resolved by qualified identity after all replacement
// shells/declarations exist, never by copying cached engine ids.
engine_bridge_result compile_mixed_cache_checkpoint(
    asIScriptEngine& engine,
    const cache& base,
    const preprocessor_options& options,
    const lexical_preprocess_result& source,
    registry_runtime* registry,
    frontend_compile_runtime& frontend_runtime,
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

// Exports every final mixed-graph module into one fresh, internally coherent
// reference-table namespace. Unchanged cache modules regain their exact
// descriptor-only metadata after structural order verification; source
// modules receive the preprocessor projection. The caller supplies the
// profile-qualified build identifier and newly generated output GUID.
engine_bridge_result export_mixed_graph_checkpoint(
    const cache& base,
    const lexical_preprocess_result& source,
    const std::vector<asIScriptModule*>& modules,
    const std::array<std::uint8_t, 16U>& data_guid,
    std::int32_t build_identifier,
    cache& output);

} // namespace gore::as::standalone::precompiled
