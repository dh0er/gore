#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace gore::as::standalone::precompiled {

inline constexpr std::size_t kMaxCacheBytes = 512U * 1024U * 1024U;
inline constexpr std::size_t kMaxStringUnits = 1'000'000U;
inline constexpr std::size_t kMaxContainerElements = 50'000'000U;

struct codec_error {
    std::size_t offset = 0U;
    std::string field;
    std::string detail;
};

// FStringInArchive is an ANSI/UTF-8 byte string selected by its owning field.
// Keeping bytes rather than transcoding makes parse -> encode byte exact.
struct archive_string {
    std::string bytes;

    friend bool operator==(const archive_string& left, const archive_string& right) noexcept {
        return left.bytes == right.bytes;
    }
};

// UE FString is used only as the serialized TMap key for Modules. Payload
// excludes the mandatory terminal NUL. UTF-16 payload bytes are little-endian.
struct map_string {
    bool utf16 = false;
    std::vector<std::uint8_t> payload;

    friend bool operator==(const map_string& left, const map_string& right) noexcept {
        return left.utf16 == right.utf16 && left.payload == right.payload;
    }
};

struct data_type {
    bool is_reference = false;
    bool is_object_const = false;
    bool is_object_handle = false;
    bool is_const_handle = false;
    bool is_auto = false;
    bool if_handle_then_const = false;
    std::int64_t type_info = 0;
    std::int32_t token_type = -1;
};

struct function_signature {
    archive_string name;
    archive_string name_space;
    std::vector<data_type> parameter_types;
    std::vector<std::int32_t> parameter_flags;
    std::vector<archive_string> parameter_default_args;
    data_type return_type;
};

struct precompiled_function {
    archive_string function_name;
    archive_string name_space;
    data_type return_type;
    std::vector<data_type> parameter_types;
    std::vector<archive_string> parameter_names;
    std::vector<std::int32_t> parameter_flags;
    std::vector<archive_string> parameter_default_args;
    std::int32_t function_traits = 0;
    std::vector<std::int32_t> byte_code;
    std::vector<std::int32_t> byte_code_references;
    std::int32_t variable_space = -1;
    std::vector<std::int64_t> object_variable_types;
    std::vector<std::int32_t> object_variable_positions;
    std::int32_t object_variables_on_heap = -1;
    std::vector<std::int32_t> variable_info_program_positions;
    std::vector<std::int32_t> variable_info_offsets;
    std::vector<std::int32_t> variable_info_options;
    std::int32_t stack_needed = -1;
    std::uint32_t id = 0U;
    std::int32_t declared_at = 0;
    std::vector<std::int32_t> line_numbers;

    bool is_unreal_function = false;
    archive_string unreal_function_name;
    std::vector<archive_string> metadata_specifiers;
    std::vector<archive_string> metadata_values;
    bool blueprint_callable = false;
    bool blueprint_override = false;
    bool blueprint_event = false;
    bool blueprint_pure = false;
    bool net_function = false;
    bool net_multicast = false;
    bool net_client = false;
    bool net_server = false;
    bool net_validate = false;
    bool unreliable = false;
    bool blueprint_authority_only = false;
    bool exec = false;
    bool can_override_event = false;
    bool dev_function = false;
    bool is_static = false;
    bool is_const_method = false;
    bool thread_safe = false;
    bool is_no_op = false;
};

struct precompiled_property {
    archive_string name;
    data_type type;
    bool is_private = false;
    bool is_protected = false;

    bool is_unreal_property = false;
    std::vector<archive_string> metadata_specifiers;
    std::vector<archive_string> metadata_values;
    bool blueprint_readable = false;
    bool blueprint_writable = false;
    bool edit_const = false;
    bool editable_on_defaults = false;
    bool editable_on_instance = false;
    bool instanced_reference = false;
    bool persistent_instance = false;
    bool advanced_display = false;
    bool transient = false;
    bool replicated = false;
    std::int32_t replication_condition = 0;
    bool skip_replication = false;
    bool skip_serialization = false;
    bool save_game = false;
    bool rep_notify = false;
    bool config = false;
    bool interp = false;
    bool asset_registry_searchable = false;
};

struct precompiled_class {
    archive_string class_name;
    archive_string name_space;
    std::int32_t flags = 0;
    std::vector<precompiled_property> properties;
    std::vector<precompiled_function> methods;
    std::vector<std::int32_t> method_table;
    std::int64_t derived_from = 0;
    std::int64_t shadow_type = 0;
    std::vector<precompiled_function> constructors;
    std::vector<std::int64_t> factory_references;
    std::vector<std::int64_t> behaviour_references;
    std::vector<precompiled_function> behaviour_functions;
    std::vector<std::int32_t> behaviour_function_types;

    bool is_in_preprocessor = false;
    archive_string super_class;
    archive_string code_super_class;
    bool super_is_code_class = false;
    bool abstract = false;
    bool transient = false;
    bool hide_dropdown = false;
    bool default_to_instanced = false;
    bool edit_inline_new = false;
    bool is_deprecated_class = false;
    archive_string config_name;
    archive_string static_class_global_variable_name;
    bool placeable = false;
    std::vector<archive_string> metadata_specifiers;
    std::vector<archive_string> metadata_values;
    archive_string compose_onto_class_name;
};

struct precompiled_enum {
    archive_string name;
    archive_string name_space;
    std::vector<archive_string> names;
    std::vector<std::int32_t> values;
};

struct precompiled_global {
    archive_string name;
    archive_string name_space;
    data_type type;
    bool is_default_init = false;
    bool is_pure_constant = false;
    std::uint64_t pure_constant_value = 0U;
    bool has_init_function = false;
    precompiled_function init_function;
};

struct function_import {
    archive_string imported_from_module;
    function_signature signature;
};

struct precompiled_module {
    archive_string module_name;
    std::vector<precompiled_function> functions;
    std::vector<precompiled_class> classes;
    std::vector<precompiled_enum> enums;
    std::vector<precompiled_global> global_variables;
    std::vector<function_import> function_imports;
    std::int64_t code_hash = 0;
    std::vector<archive_string> imported_modules;
    archive_string statics_class_name;
    std::vector<archive_string> declared_events;
    std::vector<archive_string> declared_delegates;
    archive_string script_relative_filename;
    std::vector<archive_string> post_init_functions;
};

struct type_reference {
    archive_string name;
    archive_string module;
    archive_string name_space;
    std::vector<data_type> sub_types;
};

struct function_reference {
    archive_string name;
    archive_string module;
    archive_string name_space;
    bool is_const = false;
    bool is_imported_decl = false;
    bool is_method = false;
    std::int64_t object_type = 0;
    std::vector<data_type> parameter_types;
    data_type return_type;
};

struct global_reference {
    archive_string name;
    archive_string module;
    archive_string name_space;
    bool is_string = false;
};

struct property_reference {
    archive_string name;
    std::int32_t old_type_id = 0;
};

struct cache {
    std::array<std::uint8_t, 16U> data_guid{};
    std::int32_t build_identifier = -1;
    std::vector<std::pair<map_string, precompiled_module>> modules;
    std::vector<std::pair<std::int64_t, type_reference>> type_references;
    std::vector<std::pair<std::int32_t, std::int64_t>> type_id_reference_to_pointer;
    std::vector<std::pair<std::int64_t, function_reference>> function_references;
    std::vector<std::pair<std::int32_t, std::int64_t>> function_id_reference_to_pointer;
    std::vector<std::pair<std::int64_t, global_reference>> global_references;
    std::vector<archive_string> static_names;
    std::vector<std::pair<std::int64_t, property_reference>> property_references;
};

// Decode validates the complete nested schema, canonical archive bools and
// terminal NULs, enforces all bounds, and requires exact EOF.
bool decode(
    const std::uint8_t* bytes,
    std::size_t size,
    cache& output,
    codec_error& error) noexcept;

// Encode validates the same bounds and conditionals before producing bytes.
// The output is replaced only on success.
bool encode(const cache& input, std::vector<std::uint8_t>& output, codec_error& error) noexcept;

} // namespace gore::as::standalone::precompiled
