#pragma once

#include "angelscript.h"

#include <cstddef>

namespace gore::as::standalone {

enum class graph_build_phase {
    none,
    input_validation,
    request_build,
    prepare_engine,
    parse_scripts,
    generate_types,
    generate_functions,
    layout_classes,
    calculate_template_sizes,
    layout_functions,
    compile_code,
    validate_template_instances,
    initialize_globals,
};

inline constexpr std::size_t no_failed_module = static_cast<std::size_t>(-1);

struct graph_build_result {
    int code = asSUCCESS;
    graph_build_phase phase = graph_build_phase::none;
    std::size_t failed_module = no_failed_module;

    [[nodiscard]] bool succeeded() const noexcept { return code >= 0; }
};

// Build one engine-local module graph under a single RequestBuild/BuildCompleted
// session. Every phase completes for the graph before the next phase begins.
// Cross-module visibility must already be configured through ImportModule.
graph_build_result build_module_graph(
    asIScriptModule* const* modules,
    std::size_t module_count);

// Compatibility wrapper. It uses the same graph orchestrator with one module.
int build_module(asIScriptModule& module);

} // namespace gore::as::standalone
