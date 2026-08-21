#pragma once

#include "angelscript.h"

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace gore::as::standalone {

enum class engine_property {
    allow_unsafe_references,
    optimize_bytecode,
    copy_script_sections,
    max_stack_size,
    use_character_literals,
    allow_multiline_strings,
    allow_implicit_handle_types,
    build_without_line_cues,
    init_global_vars_after_build,
    require_enum_scope,
    script_scanner,
    include_jit_instructions,
    string_encoding,
    property_accessor_mode,
    expand_default_array_to_template,
    auto_garbage_collect,
    disallow_global_vars,
    always_implement_default_construct,
    compiler_warnings,
    disallow_value_assign_for_reference_type,
    alter_syntax_named_args,
    disable_integer_division,
    disallow_empty_list_elements,
    private_property_as_protected,
    allow_unicode_identifiers,
    heredoc_trim_mode,
    max_nested_calls,
    generic_call_mode,
    automatic_imports,
    typecheck_switch_enums,
    allow_double_type,
    float_is_float64,
    warn_on_float_constants_for_doubles,
    warn_integer_division,
};

struct engine_property_setting {
    std::uint32_t ordinal = 0U;
    engine_property property = engine_property::allow_unsafe_references;
    std::uintptr_t value = 0U;
};

enum class call_convention {
    cdecl_call,
    stdcall_call,
    thiscall_as_global,
    thiscall,
    cdecl_object_last,
    cdecl_object_first,
    generic,
    thiscall_object_last,
    thiscall_object_first,
};

enum class object_behaviour {
    construct,
    list_construct,
    destruct,
    factory,
    list_factory,
    add_ref,
    release,
    get_weakref_flag,
    template_callback,
    get_ref_count,
    set_gc_flag,
    get_gc_flag,
    enum_refs,
    release_refs,
};

enum class template_validation_adapter {
    none,
    t_array,
    t_map,
    t_set,
    t_optional,
    t_subclass_of,
    t_object_ptr,
    t_weak_object_ptr,
    t_soft_object_ptr,
    t_soft_class_ptr,
};

enum class primitive_type {
    bool_type,
    int8,
    int16,
    int32,
    int64,
    uint8,
    uint16,
    uint32,
    uint64,
    float32,
    float64,
};

struct fixed_type_operations {
    bool can_be_template_subtype = false;
    bool can_construct = false;
    bool need_construct = true;
    bool can_destruct = false;
    bool need_destruct = true;
    bool can_copy = false;
    bool need_copy = true;
    bool can_compare = false;
    bool can_hash_value = false;
    std::uint32_t value_size = 0U;
    std::uint32_t value_alignment = 1U;
    bool is_object_pointer = false;
};

struct primitive_type_operations {
    std::uint32_t ordinal = 0U;
    primitive_type primitive = primitive_type::bool_type;
    fixed_type_operations operations;
};

struct dynamic_script_type_operations {
    fixed_type_operations delegate;
    fixed_type_operations multicast_delegate;
};

enum class dynamic_script_type_category { script_struct, delegate, multicast_delegate };

enum class type_operations_kind { unavailable, fixed, t_array, t_map, t_set, t_optional };

struct type_operations {
    type_operations_kind kind = type_operations_kind::unavailable;
    fixed_type_operations fixed;
};

enum class host_stub_kind { callable, storage, object };

struct host_stub {
    std::uint32_t stub_id = 0U;
    host_stub_kind kind = host_stub_kind::callable;
    std::uint32_t byte_len = 0U;
    std::uint32_t alignment = 1U;
};

struct registration_context {
    std::string name_space;
    std::optional<std::string> config_group;
    std::uint32_t access_mask = 0xffffffffU;
};

enum class registration_kind {
    object_type,
    interface_type,
    interface_method,
    object_property,
    object_method,
    object_behaviour,
    global_property,
    global_function,
    enum_type,
    enum_value,
    funcdef,
    typedef_type,
    string_factory,
    default_array_type,
};

// A native projection of RegistrationEntryV1. Its converter must leave fields
// not used by a particular variant at their neutral values. replay_registry
// validates all behaviorally relevant fields, cross-references, result
// identities and final-state coverage before mutation.
struct registration_entry {
    std::uint32_t ordinal = 0U;
    std::uint32_t registration_id = 0U;
    registration_context context;
    registration_kind kind = registration_kind::object_type;
    std::uint32_t logical_id = 0U;
    std::uint32_t owner_type_id = 0U;
    std::string declaration;
    std::string name;
    std::string target_declaration;
    std::uint32_t byte_size = 0U;
    std::uint32_t alignment = 1U;
    std::uint32_t flags = 0U;
    std::uint32_t byte_offset = 0U;
    std::uint32_t composite_offset = 0U;
    bool is_composite_indirect = false;
    std::uint32_t accessor_type = 255U;
    bool is_protected = false;
    call_convention convention = call_convention::cdecl_call;
    object_behaviour behaviour = object_behaviour::construct;
    std::uint32_t callable_stub_id = 0U;
    std::optional<std::uint32_t> auxiliary_object_stub_id;
    template_validation_adapter validation_adapter = template_validation_adapter::none;
    std::uint32_t storage_stub_id = 0U;
    std::uint32_t factory_object_stub_id = 0U;
    std::int32_t enum_value = 0;
    type_operations operations;
};

enum class registration_result_kind {
    object_type,
    interface_type,
    interface_method,
    object_property,
    object_method,
    object_behaviour,
    global_property,
    global_function,
    enum_type,
    enum_value,
    funcdef,
    typedef_type,
    string_factory,
    default_array_type,
};

struct registration_result {
    std::uint32_t ordinal = 0U;
    std::uint32_t trace_registration_id = 0U;
    registration_result_kind kind = registration_result_kind::object_type;
    std::uint32_t engine_id = 0U;
    std::uint32_t owner_engine_type_id = 0U;
    std::uint32_t index = 0U;
    bool installed = false;
};

enum class compile_out_mode {
    compile_calls,
    compile_out_entirely,
    replace_with_first_param,
    compile_out_as_method_chain,
};

enum class first_param_metadata { none, script_function, script_object_type };
enum class post_bind_state_kind { object_type, object_property, function, global_property };

struct post_bind_state {
    post_bind_state_kind kind = post_bind_state_kind::object_type;
    std::uint32_t logical_id = 0U;

    std::uint32_t byte_size = 0U;
    std::uint32_t alignment = 1U;
    std::uint32_t flags = 0U;
    std::optional<std::uint32_t> base_type_id;
    std::optional<std::uint32_t> shadow_type_id;
    std::vector<std::uint32_t> interface_type_ids;
    std::vector<std::uint32_t> interface_vft_offsets;
    bool has_implicit_constructors = false;
    bool accepts_value_subtype = false;
    bool accepts_reference_subtype = false;
    bool is_invalid_generated_type = false;

    std::uint32_t byte_offset = 0U;
    std::uint32_t access_mask = 0xffffffffU;
    std::uint32_t composite_offset = 0U;
    bool is_composite_indirect = false;
    bool is_private = false;
    bool is_protected = false;
    bool is_app_bind_property = false;
    std::uint32_t exposed_type = 255U;

    std::uint32_t trait_bits = 0U;
    std::optional<std::uint8_t> hidden_argument_index;
    std::optional<std::string> hidden_argument_default;
    std::optional<std::uint8_t> determines_output_type_argument_index;
    compile_out_mode compile_out = compile_out_mode::compile_calls;
    first_param_metadata first_param = first_param_metadata::none;

    bool is_pure_constant = false;
    std::optional<std::uint64_t> pure_constant_value;
};

struct registry_profile {
    std::vector<engine_property_setting> engine_properties;
    std::vector<host_stub> host_stubs;
    std::vector<primitive_type_operations> primitive_operations;
    dynamic_script_type_operations dynamic_script_operations;
    std::vector<registration_entry> registrations;
    std::vector<registration_result> expected_results;
    std::vector<post_bind_state> final_states;
};

enum class registry_replay_phase {
    none,
    validate_profile,
    apply_engine_properties,
    apply_registration_context,
    register_entry,
    verify_registration_result,
    apply_post_bind_state,
    verify_post_bind_state,
};

struct registry_replay_result {
    int code = asSUCCESS;
    registry_replay_phase phase = registry_replay_phase::none;
    std::size_t failed_ordinal = static_cast<std::size_t>(-1);
    std::string detail;

    [[nodiscard]] bool succeeded() const noexcept { return code >= 0; }
};

class registry_runtime final {
public:
    class impl;

    registry_runtime();
    ~registry_runtime();
    registry_runtime(registry_runtime&&) noexcept;
    registry_runtime& operator=(registry_runtime&&) noexcept;
    registry_runtime(const registry_runtime&) = delete;
    registry_runtime& operator=(const registry_runtime&) = delete;

private:
    std::unique_ptr<impl> impl_;
    friend registry_replay_result replay_registry(
        asIScriptEngine&, const registry_profile&, registry_runtime&);
    friend bool classify_dynamic_script_type(
        registry_runtime&, asITypeInfo&, dynamic_script_type_category);
};

// The engine and runtime must both be fresh. The runtime must outlive the
// engine after success. The complete profile is preflighted before the first
// engine mutation; an AngelScript rejection can still leave a partial
// registry, so callers must destroy that engine rather than retrying it.
registry_replay_result replay_registry(
    asIScriptEngine& engine,
    const registry_profile& profile,
    registry_runtime& runtime);

// The future class-generator/precompiled-loader adapter must call this when a
// script value type carries G1R's delegate/multicast user-data tag. Untagged
// script values are resolved as ordinary script structs. Unknown non-null
// user data is rejected by the template validators rather than guessed.
bool classify_dynamic_script_type(
    registry_runtime& runtime,
    asITypeInfo& type,
    dynamic_script_type_category category);

} // namespace gore::as::standalone
