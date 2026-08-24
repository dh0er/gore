#pragma once

#include "gore_as_standalone/core.hpp"
#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/precompiled_data.hpp"
#include "gore_as_standalone/registry_profile.hpp"

#include "angelscript.h"

#include <cstddef>
#include <memory>
#include <string>
#include <vector>

namespace gore::as::standalone {

enum class frontend_compile_phase {
    none,
    preflight,
    create_modules,
    attach_preclass_data,
    import_modules,
    add_sections,
    build_graph,
    cleanup,
};

struct frontend_compile_result {
    int code = asSUCCESS;
    frontend_compile_phase phase = frontend_compile_phase::none;
    std::size_t module_index = no_failed_module;
    graph_build_result graph;
    std::string detail;

    [[nodiscard]] bool succeeded() const noexcept { return code >= 0; }
};

struct base_descriptor_result {
    bool ok = true;
    std::size_t module_index = no_failed_module;
    std::string detail;
};

// Extracts only authoritative preprocessor class ancestry from decoded cache
// modules. Delegate-generated and ordinary non-reflected engine types are not
// invented as source class descriptors.
base_descriptor_result derive_preprocessor_base_modules(
    const precompiled::cache& input,
    std::vector<preprocessor_base_module>& modules);

// Owns the two compiler-side delegate tag identities. It must outlive the
// engine because generated value types retain these tag pointers.
class frontend_compile_runtime final {
public:
    frontend_compile_runtime();
    ~frontend_compile_runtime();
    frontend_compile_runtime(frontend_compile_runtime&&) noexcept;
    frontend_compile_runtime& operator=(frontend_compile_runtime&&) noexcept;
    frontend_compile_runtime(const frontend_compile_runtime&) = delete;
    frontend_compile_runtime& operator=(const frontend_compile_runtime&) = delete;

    // Stable process-local identities used by both the source-only and mixed
    // cache/source frontends. Generated type metadata retains these pointers,
    // so the runtime must outlive the engine.
    [[nodiscard]] void* delegate_tag(bool multicast) noexcept;

private:
    struct impl;
    std::unique_ptr<impl> impl_;
    friend frontend_compile_result compile_preprocessed_module_graph(
        asIScriptEngine&,
        const preprocessor_options&,
        const lexical_preprocess_result&,
        registry_runtime*,
        frontend_compile_runtime&,
        std::vector<asIScriptModule*>&);
};

// Materializes a successful preprocessor result in a registry-replayed engine,
// attaches G1R shadow/delegate pre-class data, imports explicit dependencies,
// and runs the graph-wide manager barriers. Precompiled base modules must
// already exist in the engine; edited base modules must have been omitted.
frontend_compile_result compile_preprocessed_module_graph(
    asIScriptEngine& engine,
    const preprocessor_options& options,
    const lexical_preprocess_result& input,
    registry_runtime* registry,
    frontend_compile_runtime& runtime,
    std::vector<asIScriptModule*>& modules);

} // namespace gore::as::standalone
