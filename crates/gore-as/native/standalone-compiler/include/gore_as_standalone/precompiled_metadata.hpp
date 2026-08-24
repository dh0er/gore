#pragma once

#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/precompiled_data.hpp"

#include <string>
#include <vector>

namespace gore::as::standalone::precompiled {

struct metadata_projection_result {
    bool ok = true;
    std::string detail;
    bool is_compile_diagnostic = false;
    std::string diagnostic_source;
    std::uint32_t diagnostic_line = 0U;
    std::uint32_t diagnostic_column = 0U;
};

struct class_generator_property_capabilities {
    std::string property_name;
    std::string type_declaration;
    bool can_create_property = false;
    bool never_requires_gc = false;
    bool requires_property = false;
};

struct class_generator_class_capabilities {
    std::string class_name;
    std::string name_space;
    std::vector<class_generator_property_capabilities> properties;
};

struct class_generator_capability_table {
    std::vector<class_generator_class_capabilities> classes;
};

// Applies the source-only FAngelscriptModuleDesc fields that cannot be
// recovered from asCModule. The update is atomic and rejects any reflected
// class/property/function descriptor that does not map 1:1 to the compiled
// engine record. ComposeOnto is copied only when an external ClassAnalyze hook
// supplied it; this layer never invents that game-specific result.
metadata_projection_result project_preprocessed_metadata(
    const lexical_module_description& description,
    precompiled_module& module,
    bool mark_non_uproperty_properties_as_transient = false,
    const class_generator_capability_table* capabilities = nullptr);

// Restores descriptor-only fields from an unchanged cached module onto a
// freshly exported engine record. Positional identity is accepted only after
// all class/property/function names and counts match exactly.
metadata_projection_result preserve_cached_metadata(
    const precompiled_module& cached,
    precompiled_module& rebuilt);

} // namespace gore::as::standalone::precompiled
