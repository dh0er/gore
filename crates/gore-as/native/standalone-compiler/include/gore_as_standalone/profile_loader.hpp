#pragma once

#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/registry_profile.hpp"
#include "gore_as_standalone/sha256.hpp"

#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace gore::as::standalone {

struct sealed_blob {
    std::string path;
    std::uint64_t byte_len = 0U;
    sha256_digest sha256{};
};

struct compiler_profile_manifest {
    sha256_digest profile_sha256{};
    std::uint64_t steam_build_id = 0U;
    std::uint32_t depot_id = 0U;
    std::uint64_t depot_manifest_gid = 0U;
    std::uint32_t as_create_version = 0U;
    std::uint64_t registration_trace_count = 0U;
    std::int32_t build_identifier = -1;
    std::string required_probe_suite_version;
    sealed_blob oracle_binds_cache;
    sealed_blob oracle_shipping_cache;
    sealed_blob ordered_engine_properties;
    sealed_blob registration_trace;
    sealed_blob post_bind_snapshot;
    sealed_blob preprocessor_config;
    sealed_blob class_generator_config;
    sealed_blob compiler_options;
    std::vector<sealed_blob> all_blobs;
};

struct compiler_options {
    bool mark_non_uproperty_properties_as_transient = false;
    bool error_on_incorrect_editor_only_code = false;
    bool warn_on_divergent_comparison_operator_overloads = false;
    bool warn_on_implicit_signed_unsigned_conversion = false;
    bool warn_on_increment_decrement_in_complex_expression = false;
    bool warn_on_unused_return_value_for_const_methods = false;
};

// Exact, sealed observations for donor extension points whose implementations
// live outside UNREANGEL. A bound hook rejects any invocation which has no
// matching capture; an unbound hook must carry no captures.
struct class_analyze_capture {
    std::uint32_t ordinal = 0U;
    std::string module_name;
    std::string name_space;
    std::string class_name;
    sha256_digest source_sha256{};
    sha256_digest input_generated_statics_sha256{};
    std::string generated_statics;
    sha256_digest output_generated_statics_sha256{};
    bool has_statics = false;
    std::string compose_onto_class;
};

struct graph_hook_module_capture {
    std::uint32_t ordinal = 0U;
    std::string module_name;
    std::string generated_declarations;
};

struct graph_hook_capture {
    std::uint32_t ordinal = 0U;
    sha256_digest input_graph_sha256{};
    sha256_digest output_graph_sha256{};
    std::vector<graph_hook_module_capture> modules;
};

struct graph_hook_profile {
    bool bound = false;
    std::vector<graph_hook_capture> captures;
};

struct external_frontend_profile {
    bool class_analyze_bound = false;
    std::vector<class_analyze_capture> class_analyze_captures;
    graph_hook_profile process_chunks;
    graph_hook_profile post_process_code;
};

bool parse_compiler_profile_manifest(
    std::string_view bytes,
    compiler_profile_manifest& output,
    std::string& detail);

bool parse_registry_profile_payloads(
    std::string_view ordered_properties_json,
    std::string_view registration_trace_json,
    std::string_view post_bind_snapshot_json,
    std::uint64_t expected_trace_count,
    registry_profile& output,
    std::string& detail);

bool parse_frontend_profile_payloads(
    std::string_view preprocessor_json,
    std::string_view class_generator_json,
    std::string_view compiler_options_json,
    preprocessor_options& preprocessor,
    compiler_options& compiler,
    external_frontend_profile& external_frontend,
    std::string& detail);

} // namespace gore::as::standalone
