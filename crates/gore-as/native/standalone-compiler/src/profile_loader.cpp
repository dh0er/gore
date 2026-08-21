#include "gore_as_standalone/profile_loader.hpp"

#include "gore_as_standalone/json.hpp"

#include <algorithm>
#include <array>
#include <cctype>
#include <charconv>
#include <cstring>
#include <exception>
#include <limits>
#include <map>
#include <optional>
#include <set>

namespace gore::as::standalone {
namespace {

using json::value;
using json::value_kind;

constexpr std::size_t max_json_depth = 32U;
constexpr std::string_view profile_hash_domain{
    "gore-as-compiler-profile-v1\0",
    sizeof("gore-as-compiler-profile-v1\0") - 1U};

bool parsed(std::string_view bytes, value& output, std::string& detail) {
    json::parse_error error;
    if (!json::parse(bytes, max_json_depth, output, error)) {
        detail = "JSON offset " + std::to_string(error.offset) + ": " + error.detail;
        return false;
    }
    return true;
}

bool u32(const value& input, std::string_view name, std::uint32_t& output, std::string& detail) {
    std::uint64_t wide = 0U;
    if (!json::get_u64(input, name, wide, detail)) return false;
    if (wide > std::numeric_limits<std::uint32_t>::max()) {
        detail = "field " + std::string(name) + " is outside uint32";
        return false;
    }
    output = static_cast<std::uint32_t>(wide);
    return true;
}

bool i32(const value& input, std::string_view name, std::int32_t& output, std::string& detail) {
    std::int64_t wide = 0;
    if (!json::get_i64(input, name, wide, detail)) return false;
    if (wide < std::numeric_limits<std::int32_t>::min() ||
        wide > std::numeric_limits<std::int32_t>::max()) {
        detail = "field " + std::string(name) + " is outside int32";
        return false;
    }
    output = static_cast<std::int32_t>(wide);
    return true;
}

bool digest_field(
    const value& input,
    std::string_view name,
    sha256_digest& output,
    std::string& detail) {
    std::string text;
    if (!json::get_string(input, name, text, detail)) return false;
    if (!parse_sha256_hex(text, output)) {
        detail = "field " + std::string(name) + " is not a lowercase SHA-256 digest";
        return false;
    }
    return true;
}

bool string_enum(
    const value& input,
    std::string_view name,
    std::string& text,
    std::string& detail) {
    return json::get_string(input, name, text, detail);
}

bool exact_schema(
    const value& input,
    const std::string_view expected,
    std::string& detail) {
    std::string schema;
    std::uint32_t version = 0U;
    if (!json::get_string(input, "schema", schema, detail) ||
        !u32(input, "schema_version", version, detail)) return false;
    if (schema != expected || version != 1U) {
        detail = "unsupported schema " + schema + " version " + std::to_string(version);
        return false;
    }
    return true;
}

bool parse_sealed_blob(const value& input, sealed_blob& output, std::string& detail) {
    if (!json::require_object_keys(input, {"path", "byte_len", "sha256"}, {}, detail) ||
        !json::get_string(input, "path", output.path, detail) ||
        !json::get_u64(input, "byte_len", output.byte_len, detail) ||
        !digest_field(input, "sha256", output.sha256, detail)) return false;
    if (output.path.empty() || output.path.size() > 512U || output.byte_len == 0U ||
        output.path.front() == '/' || output.path.find('\\') != std::string::npos ||
        output.path.find(':') != std::string::npos || output.path.find('\0') != std::string::npos) {
        detail = "profile blob path or length is invalid";
        return false;
    }
    std::size_t begin = 0U;
    while (begin <= output.path.size()) {
        const std::size_t end = output.path.find('/', begin);
        const std::string_view part(
            output.path.data() + begin,
            (end == std::string::npos ? output.path.size() : end) - begin);
        if (part.empty() || part == "." || part == ".." || part.size() > 128U ||
            part.back() == '.' || part.back() == ' ' ||
            !std::all_of(part.begin(), part.end(), [](const unsigned char ch) {
                return std::isalnum(ch) != 0 || ch == '-' || ch == '_' || ch == '.';
            })) {
            detail = "unsafe profile blob path component";
            return false;
        }
        if (end == std::string::npos) break;
        begin = end + 1U;
    }
    return true;
}

bool find_blob(
    const value& owner,
    std::string_view name,
    sealed_blob& output,
    std::vector<sealed_blob>& all,
    std::string& detail) {
    const value* member = nullptr;
    if (!json::get_object(owner, name, member, detail) || !parse_sealed_blob(*member, output, detail)) {
        detail = std::string(name) + ": " + detail;
        return false;
    }
    all.push_back(output);
    return true;
}

bool add_blob(
    const value& owner,
    std::string_view name,
    std::vector<sealed_blob>& all,
    std::string& detail) {
    sealed_blob ignored;
    return find_blob(owner, name, ignored, all, detail);
}

bool fixed_operations(const value& input, fixed_type_operations& output, std::string& detail) {
    if (!json::require_object_keys(
            input,
            {"can_be_template_subtype", "can_construct", "need_construct", "can_destruct",
             "need_destruct", "can_copy", "need_copy", "can_compare", "can_hash_value",
             "value_size", "value_alignment", "is_object_pointer"},
            {}, detail) ||
        !json::get_bool(input, "can_be_template_subtype", output.can_be_template_subtype, detail) ||
        !json::get_bool(input, "can_construct", output.can_construct, detail) ||
        !json::get_bool(input, "need_construct", output.need_construct, detail) ||
        !json::get_bool(input, "can_destruct", output.can_destruct, detail) ||
        !json::get_bool(input, "need_destruct", output.need_destruct, detail) ||
        !json::get_bool(input, "can_copy", output.can_copy, detail) ||
        !json::get_bool(input, "need_copy", output.need_copy, detail) ||
        !json::get_bool(input, "can_compare", output.can_compare, detail) ||
        !json::get_bool(input, "can_hash_value", output.can_hash_value, detail) ||
        !u32(input, "value_size", output.value_size, detail) ||
        !u32(input, "value_alignment", output.value_alignment, detail) ||
        !json::get_bool(input, "is_object_pointer", output.is_object_pointer, detail)) return false;
    return true;
}

bool type_operations_value(const value& input, type_operations& output, std::string& detail) {
    std::string kind;
    if (!json::get_string(input, "kind", kind, detail)) return false;
    if (kind == "unavailable") {
        if (!json::require_object_keys(input, {"kind"}, {}, detail)) return false;
        output.kind = type_operations_kind::unavailable;
    } else if (kind == "fixed") {
        const value* operations = nullptr;
        if (!json::require_object_keys(input, {"kind", "operations"}, {}, detail) ||
            !json::get_object(input, "operations", operations, detail) ||
            !fixed_operations(*operations, output.fixed, detail)) return false;
        output.kind = type_operations_kind::fixed;
    } else if (kind == "t_array" || kind == "t_map" || kind == "t_set" || kind == "t_optional") {
        if (!json::require_object_keys(input, {"kind"}, {}, detail)) return false;
        if (kind == "t_array") output.kind = type_operations_kind::t_array;
        if (kind == "t_map") output.kind = type_operations_kind::t_map;
        if (kind == "t_set") output.kind = type_operations_kind::t_set;
        if (kind == "t_optional") output.kind = type_operations_kind::t_optional;
    } else {
        detail = "unknown type operations kind " + kind;
        return false;
    }
    return true;
}

bool registration_context_value(
    const value& input,
    registration_context& output,
    std::string& detail) {
    if (!json::require_object_keys(input, {"namespace", "config_group", "access_mask"}, {}, detail) ||
        !json::get_string(input, "namespace", output.name_space, detail) ||
        !u32(input, "access_mask", output.access_mask, detail)) return false;
    bool present = false;
    std::string config;
    if (!json::get_optional_string(input, "config_group", present, config, detail)) return false;
    output.config_group = present ? std::optional<std::string>(std::move(config)) : std::nullopt;
    return true;
}

template <typename Enum>
bool map_enum(
    const std::string& text,
    const std::initializer_list<std::pair<std::string_view, Enum>> values,
    Enum& output,
    std::string& detail,
    const char* const label) {
    for (const auto& entry : values) {
        if (text == entry.first) { output = entry.second; return true; }
    }
    detail = std::string("unknown ") + label + " " + text;
    return false;
}

bool call_convention_value(const value& input, call_convention& output, std::string& detail) {
    std::string text;
    if (!string_enum(input, "call_convention", text, detail)) return false;
    return map_enum(text, {
        {"cdecl", call_convention::cdecl_call}, {"stdcall", call_convention::stdcall_call},
        {"thiscall_as_global", call_convention::thiscall_as_global},
        {"thiscall", call_convention::thiscall},
        {"cdecl_object_last", call_convention::cdecl_object_last},
        {"cdecl_object_first", call_convention::cdecl_object_first},
        {"generic", call_convention::generic},
        {"thiscall_object_last", call_convention::thiscall_object_last},
        {"thiscall_object_first", call_convention::thiscall_object_first},
    }, output, detail, "call convention");
}

bool behaviour_value(const value& input, object_behaviour& output, std::string& detail) {
    std::string text;
    if (!string_enum(input, "behaviour", text, detail)) return false;
    return map_enum(text, {
        {"construct", object_behaviour::construct}, {"list_construct", object_behaviour::list_construct},
        {"destruct", object_behaviour::destruct}, {"factory", object_behaviour::factory},
        {"list_factory", object_behaviour::list_factory}, {"add_ref", object_behaviour::add_ref},
        {"release", object_behaviour::release}, {"get_weakref_flag", object_behaviour::get_weakref_flag},
        {"template_callback", object_behaviour::template_callback},
        {"get_ref_count", object_behaviour::get_ref_count}, {"set_gc_flag", object_behaviour::set_gc_flag},
        {"get_gc_flag", object_behaviour::get_gc_flag}, {"enum_refs", object_behaviour::enum_refs},
        {"release_refs", object_behaviour::release_refs},
    }, output, detail, "object behaviour");
}

bool template_adapter_value(
    const value& input,
    template_validation_adapter& output,
    std::string& detail) {
    const value* member = input.find("template_validation_adapter");
    if (member == nullptr || member->kind == value_kind::null_value) {
        output = template_validation_adapter::none;
        return true;
    }
    if (member->kind != value_kind::string) {
        detail = "template_validation_adapter has the wrong JSON type";
        return false;
    }
    return map_enum(member->text, {
        {"t_array", template_validation_adapter::t_array},
        {"t_map", template_validation_adapter::t_map},
        {"t_set", template_validation_adapter::t_set},
        {"t_optional", template_validation_adapter::t_optional},
        {"t_subclass_of", template_validation_adapter::t_subclass_of},
        {"t_object_ptr", template_validation_adapter::t_object_ptr},
        {"t_weak_object_ptr", template_validation_adapter::t_weak_object_ptr},
        {"t_soft_object_ptr", template_validation_adapter::t_soft_object_ptr},
        {"t_soft_class_ptr", template_validation_adapter::t_soft_class_ptr},
    }, output, detail, "template validation adapter");
}

bool optional_u32(
    const value& input,
    std::string_view name,
    std::optional<std::uint32_t>& output,
    std::string& detail) {
    bool present = false;
    std::uint64_t wide = 0U;
    if (!json::get_optional_u64(input, name, present, wide, detail)) return false;
    if (!present) { output.reset(); return true; }
    if (wide > std::numeric_limits<std::uint32_t>::max()) {
        detail = "field " + std::string(name) + " is outside uint32";
        return false;
    }
    output = static_cast<std::uint32_t>(wide);
    return true;
}

bool common_registration(
    const value& input,
    registration_entry& output,
    std::string& detail) {
    const value* context = nullptr;
    return u32(input, "ordinal", output.ordinal, detail) &&
        u32(input, "registration_id", output.registration_id, detail) &&
        json::get_object(input, "context", context, detail) &&
        registration_context_value(*context, output.context, detail);
}

bool parse_registration(const value& input, registration_entry& output, std::string& detail) {
    std::string kind;
    if (!json::get_string(input, "kind", kind, detail) || !common_registration(input, output, detail)) {
        return false;
    }
    if (kind == "object_type") {
        const value* operations = nullptr;
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_id", "declaration",
                 "byte_size", "alignment", "flags", "type_operations"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !u32(input, "byte_size", output.byte_size, detail) ||
            !u32(input, "alignment", output.alignment, detail) ||
            !u32(input, "flags", output.flags, detail) ||
            !json::get_object(input, "type_operations", operations, detail) ||
            !type_operations_value(*operations, output.operations, detail)) return false;
        output.kind = registration_kind::object_type;
    } else if (kind == "interface") {
        const value* operations = nullptr;
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_id", "declaration", "type_operations"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !json::get_object(input, "type_operations", operations, detail) ||
            !type_operations_value(*operations, output.operations, detail)) return false;
        output.kind = registration_kind::interface_type;
    } else if (kind == "interface_method") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "function_id", "owner_type_id", "declaration"}, {}, detail) ||
            !u32(input, "function_id", output.logical_id, detail) ||
            !u32(input, "owner_type_id", output.owner_type_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail)) return false;
        output.kind = registration_kind::interface_method;
    } else if (kind == "object_property") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "property_id", "owner_type_id", "declaration",
                 "byte_offset", "composite_offset", "is_composite_indirect", "accessor_type", "is_protected"}, {}, detail) ||
            !u32(input, "property_id", output.logical_id, detail) ||
            !u32(input, "owner_type_id", output.owner_type_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !u32(input, "byte_offset", output.byte_offset, detail) ||
            !u32(input, "composite_offset", output.composite_offset, detail) ||
            !json::get_bool(input, "is_composite_indirect", output.is_composite_indirect, detail) ||
            !u32(input, "accessor_type", output.accessor_type, detail) ||
            !json::get_bool(input, "is_protected", output.is_protected, detail)) return false;
        output.kind = registration_kind::object_property;
    } else if (kind == "object_method") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "function_id", "owner_type_id", "declaration",
                 "call_convention", "callable_stub_id", "auxiliary_object_stub_id", "composite_offset",
                 "is_composite_indirect", "accessor_type"}, {}, detail) ||
            !u32(input, "function_id", output.logical_id, detail) ||
            !u32(input, "owner_type_id", output.owner_type_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !call_convention_value(input, output.convention, detail) ||
            !u32(input, "callable_stub_id", output.callable_stub_id, detail) ||
            !optional_u32(input, "auxiliary_object_stub_id", output.auxiliary_object_stub_id, detail) ||
            !u32(input, "composite_offset", output.composite_offset, detail) ||
            !json::get_bool(input, "is_composite_indirect", output.is_composite_indirect, detail) ||
            !u32(input, "accessor_type", output.accessor_type, detail)) return false;
        output.kind = registration_kind::object_method;
    } else if (kind == "object_behaviour") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "function_id", "owner_type_id", "behaviour",
                 "declaration", "call_convention", "callable_stub_id", "auxiliary_object_stub_id",
                 "template_validation_adapter", "composite_offset", "is_composite_indirect"}, {}, detail) ||
            !u32(input, "function_id", output.logical_id, detail) ||
            !u32(input, "owner_type_id", output.owner_type_id, detail) ||
            !behaviour_value(input, output.behaviour, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !call_convention_value(input, output.convention, detail) ||
            !u32(input, "callable_stub_id", output.callable_stub_id, detail) ||
            !optional_u32(input, "auxiliary_object_stub_id", output.auxiliary_object_stub_id, detail) ||
            !template_adapter_value(input, output.validation_adapter, detail) ||
            !u32(input, "composite_offset", output.composite_offset, detail) ||
            !json::get_bool(input, "is_composite_indirect", output.is_composite_indirect, detail)) return false;
        output.kind = registration_kind::object_behaviour;
    } else if (kind == "global_property") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "property_id", "declaration", "storage_stub_id"}, {}, detail) ||
            !u32(input, "property_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !u32(input, "storage_stub_id", output.storage_stub_id, detail)) return false;
        output.kind = registration_kind::global_property;
    } else if (kind == "global_function") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "function_id", "declaration", "call_convention",
                 "callable_stub_id", "auxiliary_object_stub_id"}, {}, detail) ||
            !u32(input, "function_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !call_convention_value(input, output.convention, detail) ||
            !u32(input, "callable_stub_id", output.callable_stub_id, detail) ||
            !optional_u32(input, "auxiliary_object_stub_id", output.auxiliary_object_stub_id, detail)) return false;
        output.kind = registration_kind::global_function;
    } else if (kind == "enum") {
        const value* operations = nullptr;
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_id", "declaration", "type_operations"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !json::get_object(input, "type_operations", operations, detail) ||
            !type_operations_value(*operations, output.operations, detail)) return false;
        output.kind = registration_kind::enum_type;
    } else if (kind == "enum_value") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "owner_type_id", "name", "value"}, {}, detail) ||
            !u32(input, "owner_type_id", output.owner_type_id, detail) ||
            !json::get_string(input, "name", output.name, detail) ||
            !i32(input, "value", output.enum_value, detail)) return false;
        output.kind = registration_kind::enum_value;
    } else if (kind == "funcdef") {
        const value* operations = nullptr;
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_id", "declaration", "type_operations"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !json::get_string(input, "declaration", output.declaration, detail) ||
            !json::get_object(input, "type_operations", operations, detail) ||
            !type_operations_value(*operations, output.operations, detail)) return false;
        output.kind = registration_kind::funcdef;
    } else if (kind == "typedef") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_id", "name", "target_declaration"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !json::get_string(input, "name", output.name, detail) ||
            !json::get_string(input, "target_declaration", output.target_declaration, detail)) return false;
        output.kind = registration_kind::typedef_type;
    } else if (kind == "string_factory") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "string_type_declaration", "factory_object_stub_id"}, {}, detail) ||
            !json::get_string(input, "string_type_declaration", output.declaration, detail) ||
            !u32(input, "factory_object_stub_id", output.factory_object_stub_id, detail)) return false;
        output.kind = registration_kind::string_factory;
    } else if (kind == "default_array_type") {
        if (!json::require_object_keys(input,
                {"kind", "ordinal", "registration_id", "context", "type_declaration"}, {}, detail) ||
            !json::get_string(input, "type_declaration", output.declaration, detail)) return false;
        output.kind = registration_kind::default_array_type;
    } else {
        detail = "unknown registration kind " + kind;
        return false;
    }
    return true;
}

bool engine_property_value(const std::string& text, engine_property& output, std::string& detail) {
    return map_enum(text, {
        {"allow_unsafe_references", engine_property::allow_unsafe_references},
        {"optimize_bytecode", engine_property::optimize_bytecode},
        {"copy_script_sections", engine_property::copy_script_sections},
        {"max_stack_size", engine_property::max_stack_size},
        {"use_character_literals", engine_property::use_character_literals},
        {"allow_multiline_strings", engine_property::allow_multiline_strings},
        {"allow_implicit_handle_types", engine_property::allow_implicit_handle_types},
        {"build_without_line_cues", engine_property::build_without_line_cues},
        {"init_global_vars_after_build", engine_property::init_global_vars_after_build},
        {"require_enum_scope", engine_property::require_enum_scope},
        {"script_scanner", engine_property::script_scanner},
        {"include_jit_instructions", engine_property::include_jit_instructions},
        {"string_encoding", engine_property::string_encoding},
        {"property_accessor_mode", engine_property::property_accessor_mode},
        {"expand_default_array_to_template", engine_property::expand_default_array_to_template},
        {"auto_garbage_collect", engine_property::auto_garbage_collect},
        {"disallow_global_vars", engine_property::disallow_global_vars},
        {"always_implement_default_construct", engine_property::always_implement_default_construct},
        {"compiler_warnings", engine_property::compiler_warnings},
        {"disallow_value_assign_for_ref_type", engine_property::disallow_value_assign_for_reference_type},
        {"alter_syntax_named_args", engine_property::alter_syntax_named_args},
        {"disable_integer_division", engine_property::disable_integer_division},
        {"disallow_empty_list_elements", engine_property::disallow_empty_list_elements},
        {"private_property_as_protected", engine_property::private_property_as_protected},
        {"allow_unicode_identifiers", engine_property::allow_unicode_identifiers},
        {"heredoc_trim_mode", engine_property::heredoc_trim_mode},
        {"max_nested_calls", engine_property::max_nested_calls},
        {"generic_call_mode", engine_property::generic_call_mode},
        {"automatic_imports", engine_property::automatic_imports},
        {"typecheck_switch_enums", engine_property::typecheck_switch_enums},
        {"allow_double_type", engine_property::allow_double_type},
        {"float_is_float64", engine_property::float_is_float64},
        {"warn_on_float_constants_for_doubles", engine_property::warn_on_float_constants_for_doubles},
        {"warn_integer_division", engine_property::warn_integer_division},
    }, output, detail, "engine property");
}

bool parse_properties(const value& root, registry_profile& output, sha256_digest& identity, std::string& detail) {
    const value* settings = nullptr;
    if (!json::require_object_keys(root, {"schema", "schema_version", "settings", "canonical_sha256"}, {}, detail) ||
        !exact_schema(root, "gore.as.engine-properties", detail) ||
        !digest_field(root, "canonical_sha256", identity, detail) ||
        !json::get_array(root, "settings", settings, detail) || settings->elements.size() > 4096U) return false;
    output.engine_properties.clear();
    output.engine_properties.reserve(settings->elements.size());
    for (const auto& item : settings->elements) {
        engine_property_setting setting;
        std::string property;
        std::uint64_t setting_value = 0U;
        if (!json::require_object_keys(item, {"ordinal", "property", "value"}, {}, detail) ||
            !u32(item, "ordinal", setting.ordinal, detail) ||
            !json::get_string(item, "property", property, detail) ||
            !engine_property_value(property, setting.property, detail) ||
            !json::get_u64(item, "value", setting_value, detail) ||
            setting_value > std::numeric_limits<std::uintptr_t>::max()) return false;
        setting.value = static_cast<std::uintptr_t>(setting_value);
        output.engine_properties.push_back(setting);
    }
    return true;
}

bool host_stub_value(const value& input, host_stub& output, std::string& detail) {
    const value* descriptor = nullptr;
    std::string purpose;
    std::string kind;
    if (!json::require_object_keys(input, {"stub_id", "purpose", "descriptor"}, {}, detail) ||
        !u32(input, "stub_id", output.stub_id, detail) ||
        !json::get_string(input, "purpose", purpose, detail) ||
        purpose != "compile_only_never_invoke" ||
        !json::get_object(input, "descriptor", descriptor, detail) ||
        !json::get_string(*descriptor, "kind", kind, detail)) {
        if (purpose != "compile_only_never_invoke" && detail.empty()) detail = "unsupported host stub purpose";
        return false;
    }
    if (kind == "callable") {
        sha256_digest ignored{};
        if (!json::require_object_keys(*descriptor, {"kind", "signature_sha256"}, {}, detail) ||
            !digest_field(*descriptor, "signature_sha256", ignored, detail)) return false;
        output.kind = host_stub_kind::callable;
    } else if (kind == "storage") {
        if (!json::require_object_keys(*descriptor, {"kind", "byte_len", "alignment"}, {}, detail) ||
            !u32(*descriptor, "byte_len", output.byte_len, detail) ||
            !u32(*descriptor, "alignment", output.alignment, detail)) return false;
        output.kind = host_stub_kind::storage;
    } else if (kind == "object") {
        sha256_digest ignored{};
        if (!json::require_object_keys(*descriptor, {"kind", "interface_sha256"}, {}, detail) ||
            !digest_field(*descriptor, "interface_sha256", ignored, detail)) return false;
        output.kind = host_stub_kind::object;
    } else {
        detail = "unknown host stub kind " + kind;
        return false;
    }
    return true;
}

bool primitive_value(const value& input, primitive_type_operations& output, std::string& detail) {
    const value* operations = nullptr;
    std::string primitive;
    if (!json::require_object_keys(input, {"ordinal", "primitive", "operations"}, {}, detail) ||
        !u32(input, "ordinal", output.ordinal, detail) ||
        !json::get_string(input, "primitive", primitive, detail) ||
        !map_enum(primitive, {
            {"bool", primitive_type::bool_type}, {"int8", primitive_type::int8},
            {"int16", primitive_type::int16}, {"int32", primitive_type::int32},
            {"int64", primitive_type::int64}, {"uint8", primitive_type::uint8},
            {"uint16", primitive_type::uint16}, {"uint32", primitive_type::uint32},
            {"uint64", primitive_type::uint64}, {"float32", primitive_type::float32},
            {"float64", primitive_type::float64},
        }, output.primitive, detail, "primitive type") ||
        !json::get_object(input, "operations", operations, detail) ||
        !fixed_operations(*operations, output.operations, detail)) return false;
    return true;
}

bool parse_trace(const value& root, registry_profile& output, sha256_digest& identity, std::string& detail) {
    const value* stubs = nullptr;
    const value* primitives = nullptr;
    const value* dynamic = nullptr;
    const value* entries = nullptr;
    if (!json::require_object_keys(root,
            {"schema", "schema_version", "host_stubs", "primitive_operations",
             "dynamic_script_operations", "entries", "canonical_sha256"}, {}, detail) ||
        !exact_schema(root, "gore.as.registration-trace", detail) ||
        !digest_field(root, "canonical_sha256", identity, detail) ||
        !json::get_array(root, "host_stubs", stubs, detail) ||
        !json::get_array(root, "primitive_operations", primitives, detail) ||
        !json::get_object(root, "dynamic_script_operations", dynamic, detail) ||
        !json::get_array(root, "entries", entries, detail)) return false;
    if (stubs->elements.size() > 1'000'000U || primitives->elements.size() != 11U ||
        entries->elements.empty() || entries->elements.size() > 2'000'000U) {
        detail = "registration trace count is outside the protocol bounds";
        return false;
    }
    output.host_stubs.clear(); output.host_stubs.reserve(stubs->elements.size());
    for (const auto& item : stubs->elements) {
        host_stub stub;
        if (!host_stub_value(item, stub, detail)) return false;
        output.host_stubs.push_back(stub);
    }
    output.primitive_operations.clear(); output.primitive_operations.reserve(primitives->elements.size());
    for (const auto& item : primitives->elements) {
        primitive_type_operations primitive;
        if (!primitive_value(item, primitive, detail)) return false;
        output.primitive_operations.push_back(primitive);
    }
    const value* delegate = nullptr;
    const value* multicast = nullptr;
    if (!json::require_object_keys(*dynamic, {"delegate", "multicast_delegate"}, {}, detail) ||
        !json::get_object(*dynamic, "delegate", delegate, detail) ||
        !json::get_object(*dynamic, "multicast_delegate", multicast, detail) ||
        !fixed_operations(*delegate, output.dynamic_script_operations.delegate, detail) ||
        !fixed_operations(*multicast, output.dynamic_script_operations.multicast_delegate, detail)) return false;
    output.registrations.clear(); output.registrations.reserve(entries->elements.size());
    for (const auto& item : entries->elements) {
        registration_entry entry;
        if (!parse_registration(item, entry, detail)) return false;
        output.registrations.push_back(std::move(entry));
    }
    return true;
}

bool result_value(const value& input, registration_result& output, std::string& detail) {
    std::string kind;
    if (!json::get_string(input, "kind", kind, detail)) return false;
    if (kind == "object_type" || kind == "interface" || kind == "enum" || kind == "funcdef" || kind == "typedef") {
        if (!json::require_object_keys(input, {"kind", "engine_type_id"}, {}, detail) ||
            !u32(input, "engine_type_id", output.engine_id, detail)) return false;
        if (kind == "object_type") output.kind = registration_result_kind::object_type;
        if (kind == "interface") output.kind = registration_result_kind::interface_type;
        if (kind == "enum") output.kind = registration_result_kind::enum_type;
        if (kind == "funcdef") output.kind = registration_result_kind::funcdef;
        if (kind == "typedef") output.kind = registration_result_kind::typedef_type;
    } else if (kind == "interface_method" || kind == "object_method" || kind == "object_behaviour") {
        if (!json::require_object_keys(input, {"kind", "owner_engine_type_id", "engine_function_id"}, {}, detail) ||
            !u32(input, "owner_engine_type_id", output.owner_engine_type_id, detail) ||
            !u32(input, "engine_function_id", output.engine_id, detail)) return false;
        if (kind == "interface_method") output.kind = registration_result_kind::interface_method;
        if (kind == "object_method") output.kind = registration_result_kind::object_method;
        if (kind == "object_behaviour") output.kind = registration_result_kind::object_behaviour;
    } else if (kind == "object_property") {
        if (!json::require_object_keys(input, {"kind", "owner_engine_type_id", "property_index"}, {}, detail) ||
            !u32(input, "owner_engine_type_id", output.owner_engine_type_id, detail) ||
            !u32(input, "property_index", output.index, detail)) return false;
        output.kind = registration_result_kind::object_property;
    } else if (kind == "global_property") {
        if (!json::require_object_keys(input, {"kind", "global_property_index"}, {}, detail) ||
            !u32(input, "global_property_index", output.index, detail)) return false;
        output.kind = registration_result_kind::global_property;
    } else if (kind == "global_function") {
        if (!json::require_object_keys(input, {"kind", "engine_function_id"}, {}, detail) ||
            !u32(input, "engine_function_id", output.engine_id, detail)) return false;
        output.kind = registration_result_kind::global_function;
    } else if (kind == "enum_value") {
        if (!json::require_object_keys(input, {"kind", "owner_engine_type_id", "value_index"}, {}, detail) ||
            !u32(input, "owner_engine_type_id", output.owner_engine_type_id, detail) ||
            !u32(input, "value_index", output.index, detail)) return false;
        output.kind = registration_result_kind::enum_value;
    } else if (kind == "string_factory" || kind == "default_array_type") {
        if (!json::require_object_keys(input, {"kind", "installed"}, {}, detail) ||
            !json::get_bool(input, "installed", output.installed, detail)) return false;
        output.kind = kind == "string_factory" ? registration_result_kind::string_factory :
            registration_result_kind::default_array_type;
    } else {
        detail = "unknown post-bind result kind " + kind;
        return false;
    }
    return true;
}

bool u32_array(const value& input, std::string_view name, std::vector<std::uint32_t>& output, std::string& detail) {
    const value* array = nullptr;
    if (!json::get_array(input, name, array, detail)) return false;
    output.clear(); output.reserve(array->elements.size());
    for (const auto& item : array->elements) {
        if (item.kind != value_kind::number || item.text.empty() || item.text.front() == '-') {
            detail = "field " + std::string(name) + " contains a non-uint32 value";
            return false;
        }
        std::uint64_t wide = 0U;
        const auto result = std::from_chars(item.text.data(), item.text.data() + item.text.size(), wide);
        if (result.ec != std::errc{} || result.ptr != item.text.data() + item.text.size() ||
            wide > std::numeric_limits<std::uint32_t>::max()) {
            detail = "field " + std::string(name) + " contains a value outside uint32";
            return false;
        }
        output.push_back(static_cast<std::uint32_t>(wide));
    }
    return true;
}

bool final_state_value(const value& input, post_bind_state& output, std::string& detail) {
    std::string kind;
    if (!json::get_string(input, "kind", kind, detail)) return false;
    if (kind == "object_type") {
        if (!json::require_object_keys(input,
                {"kind", "type_id", "byte_size", "alignment", "flags", "base_type_id", "shadow_type_id",
                 "interface_type_ids", "interface_vft_offsets", "has_implicit_constructors",
                 "accepts_value_subtype", "accepts_reference_subtype", "is_invalid_generated_type"}, {}, detail) ||
            !u32(input, "type_id", output.logical_id, detail) ||
            !u32(input, "byte_size", output.byte_size, detail) ||
            !u32(input, "alignment", output.alignment, detail) ||
            !u32(input, "flags", output.flags, detail) ||
            !optional_u32(input, "base_type_id", output.base_type_id, detail) ||
            !optional_u32(input, "shadow_type_id", output.shadow_type_id, detail) ||
            !u32_array(input, "interface_type_ids", output.interface_type_ids, detail) ||
            !u32_array(input, "interface_vft_offsets", output.interface_vft_offsets, detail) ||
            !json::get_bool(input, "has_implicit_constructors", output.has_implicit_constructors, detail) ||
            !json::get_bool(input, "accepts_value_subtype", output.accepts_value_subtype, detail) ||
            !json::get_bool(input, "accepts_reference_subtype", output.accepts_reference_subtype, detail) ||
            !json::get_bool(input, "is_invalid_generated_type", output.is_invalid_generated_type, detail)) return false;
        output.kind = post_bind_state_kind::object_type;
    } else if (kind == "object_property") {
        if (!json::require_object_keys(input,
                {"kind", "property_id", "byte_offset", "access_mask", "composite_offset", "is_composite_indirect",
                 "is_private", "is_protected", "is_app_bind_property", "exposed_type"}, {}, detail) ||
            !u32(input, "property_id", output.logical_id, detail) ||
            !u32(input, "byte_offset", output.byte_offset, detail) ||
            !u32(input, "access_mask", output.access_mask, detail) ||
            !u32(input, "composite_offset", output.composite_offset, detail) ||
            !json::get_bool(input, "is_composite_indirect", output.is_composite_indirect, detail) ||
            !json::get_bool(input, "is_private", output.is_private, detail) ||
            !json::get_bool(input, "is_protected", output.is_protected, detail) ||
            !json::get_bool(input, "is_app_bind_property", output.is_app_bind_property, detail) ||
            !u32(input, "exposed_type", output.exposed_type, detail)) return false;
        output.kind = post_bind_state_kind::object_property;
    } else if (kind == "function") {
        std::string compile_out;
        std::string first_param;
        bool hidden_present = false;
        std::uint64_t hidden = 0U;
        bool output_present = false;
        std::uint64_t determines = 0U;
        bool default_present = false;
        std::string hidden_default;
        if (!json::require_object_keys(input,
                {"kind", "function_id", "trait_bits", "exposed_type", "hidden_argument_index",
                 "hidden_argument_default", "determines_output_type_argument_index", "compile_out_mode",
                 "first_param_metadata"}, {}, detail) ||
            !u32(input, "function_id", output.logical_id, detail) ||
            !u32(input, "trait_bits", output.trait_bits, detail) ||
            !u32(input, "exposed_type", output.exposed_type, detail) ||
            !json::get_optional_u64(input, "hidden_argument_index", hidden_present, hidden, detail) ||
            !json::get_optional_string(input, "hidden_argument_default", default_present, hidden_default, detail) ||
            !json::get_optional_u64(input, "determines_output_type_argument_index", output_present, determines, detail) ||
            !json::get_string(input, "compile_out_mode", compile_out, detail) ||
            !json::get_string(input, "first_param_metadata", first_param, detail) ||
            hidden > 255U || determines > 255U) return false;
        if (hidden_present) output.hidden_argument_index = static_cast<std::uint8_t>(hidden);
        if (default_present) output.hidden_argument_default = std::move(hidden_default);
        if (output_present) output.determines_output_type_argument_index = static_cast<std::uint8_t>(determines);
        if (!map_enum(compile_out, {
                {"compile_calls", compile_out_mode::compile_calls},
                {"compile_out_entirely", compile_out_mode::compile_out_entirely},
                {"replace_with_first_param", compile_out_mode::replace_with_first_param},
                {"compile_out_as_method_chain", compile_out_mode::compile_out_as_method_chain},
            }, output.compile_out, detail, "compile-out mode") ||
            !map_enum(first_param, {
                {"none", first_param_metadata::none},
                {"script_function", first_param_metadata::script_function},
                {"script_object_type", first_param_metadata::script_object_type},
            }, output.first_param, detail, "first-param metadata")) return false;
        output.kind = post_bind_state_kind::function;
    } else if (kind == "global_property") {
        bool present = false;
        std::uint64_t value = 0U;
        if (!json::require_object_keys(input,
                {"kind", "property_id", "is_pure_constant", "pure_constant_value"}, {}, detail) ||
            !u32(input, "property_id", output.logical_id, detail) ||
            !json::get_bool(input, "is_pure_constant", output.is_pure_constant, detail) ||
            !json::get_optional_u64(input, "pure_constant_value", present, value, detail)) return false;
        if (present) output.pure_constant_value = value;
        output.kind = post_bind_state_kind::global_property;
    } else {
        detail = "unknown post-bind state kind " + kind;
        return false;
    }
    return true;
}

bool parse_snapshot(
    const value& root,
    registry_profile& output,
    const sha256_digest& properties_identity,
    const sha256_digest& trace_identity,
    std::string& detail) {
    sha256_digest properties_ref{};
    sha256_digest trace_ref{};
    sha256_digest ignored{};
    const value* entries = nullptr;
    const value* states = nullptr;
    if (!json::require_object_keys(root,
            {"schema", "schema_version", "engine_properties_sha256", "registration_trace_sha256",
             "entries", "final_states", "canonical_sha256"}, {}, detail) ||
        !exact_schema(root, "gore.as.post-bind-snapshot", detail) ||
        !digest_field(root, "engine_properties_sha256", properties_ref, detail) ||
        !digest_field(root, "registration_trace_sha256", trace_ref, detail) ||
        !digest_field(root, "canonical_sha256", ignored, detail) ||
        properties_ref != properties_identity || trace_ref != trace_identity ||
        !json::get_array(root, "entries", entries, detail) ||
        !json::get_array(root, "final_states", states, detail)) {
        if (properties_ref != properties_identity || trace_ref != trace_identity) {
            detail = "post-bind snapshot does not reference the loaded property/trace identities";
        }
        return false;
    }
    if (entries->elements.size() != output.registrations.size() || states->elements.size() > 2'000'000U) {
        detail = "post-bind snapshot coverage count is invalid";
        return false;
    }
    output.expected_results.clear(); output.expected_results.reserve(entries->elements.size());
    for (const auto& item : entries->elements) {
        const value* result = nullptr;
        registration_result converted;
        if (!json::require_object_keys(item, {"ordinal", "trace_registration_id", "result"}, {}, detail) ||
            !u32(item, "ordinal", converted.ordinal, detail) ||
            !u32(item, "trace_registration_id", converted.trace_registration_id, detail) ||
            !json::get_object(item, "result", result, detail) ||
            !result_value(*result, converted, detail)) return false;
        output.expected_results.push_back(std::move(converted));
    }
    output.final_states.clear(); output.final_states.reserve(states->elements.size());
    for (const auto& item : states->elements) {
        post_bind_state state;
        if (!final_state_value(item, state, detail)) return false;
        output.final_states.push_back(std::move(state));
    }
    return true;
}

bool parse_enum_static_class(const std::string& text, static_class_mode& output, std::string& detail) {
    return map_enum(text, {
        {"allowed", static_class_mode::allowed}, {"deprecated", static_class_mode::deprecated},
        {"disallowed", static_class_mode::disallowed},
    }, output, detail, "static class mode");
}

bool parse_enum_edit(const std::string& text, property_edit_specifier& output, std::string& detail) {
    return map_enum(text, {
        {"edit_anywhere", property_edit_specifier::edit_anywhere},
        {"edit_instance_only", property_edit_specifier::edit_instance_only},
        {"edit_defaults_only", property_edit_specifier::edit_defaults_only},
        {"not_editable", property_edit_specifier::not_editable},
    }, output, detail, "property edit specifier");
}

bool parse_enum_blueprint(const std::string& text, property_blueprint_specifier& output, std::string& detail) {
    return map_enum(text, {
        {"blueprint_read_write", property_blueprint_specifier::blueprint_read_write},
        {"blueprint_read_only", property_blueprint_specifier::blueprint_read_only},
        {"blueprint_hidden", property_blueprint_specifier::blueprint_hidden},
    }, output, detail, "property blueprint specifier");
}

bool parse_enum_native_super(const std::string& text, native_super_kind& output, std::string& detail) {
    return map_enum(text, {
        {"actor", native_super_kind::actor}, {"actor_component", native_super_kind::actor_component},
        {"engine_subsystem", native_super_kind::engine_subsystem},
        {"editor_subsystem", native_super_kind::editor_subsystem},
        {"game_instance_subsystem", native_super_kind::game_instance_subsystem},
        {"world_subsystem", native_super_kind::world_subsystem},
        {"local_player_subsystem", native_super_kind::local_player_subsystem},
        {"other_u_object", native_super_kind::other_uobject},
    }, output, detail, "native super kind");
}

} // namespace

bool parse_compiler_profile_manifest(
    const std::string_view bytes,
    compiler_profile_manifest& output,
    std::string& detail) {
    try {
        value root;
        if (bytes.size() > 4U * 1024U * 1024U || !parsed(bytes, root, detail) ||
            !json::require_object_keys(root,
                {"schema", "schema_version", "target", "oracle", "binds", "engine", "unreal_semantics",
                 "frontend", "bytecode", "cache_writer", "qualification", "profile_sha256"}, {}, detail) ||
            !exact_schema(root, "gore.as.compiler-profile", detail)) return false;

        const std::array<std::string_view, 12U> expected_order{{
            "schema", "schema_version", "target", "oracle", "binds", "engine", "unreal_semantics",
            "frontend", "bytecode", "cache_writer", "qualification", "profile_sha256"}};
        for (std::size_t index = 0U; index < expected_order.size(); ++index) {
            if (root.members[index].first != expected_order[index]) {
                detail = "compiler manifest is not in canonical field order";
                return false;
            }
        }

        compiler_profile_manifest staged;
        if (!digest_field(root, "profile_sha256", staged.profile_sha256, detail)) return false;
        value hash_payload = root;
        hash_payload.members.pop_back();
        std::string canonical;
        if (!json::serialize_compact(hash_payload, canonical)) {
            detail = "could not serialize canonical compiler manifest";
            return false;
        }
        sha256 hash;
        hash.update(profile_hash_domain.data(), profile_hash_domain.size());
        hash.update(canonical);
        if (hash.finish() != staged.profile_sha256) {
            detail = "compiler manifest profile_sha256 mismatch";
            return false;
        }

        const value* target = nullptr;
        if (!json::get_object(root, "target", target, detail) ||
            !json::require_object_keys(*target,
                {"steam_app_id", "steam_build_id", "depot_id", "depot_manifest_gid", "platform",
                 "architecture", "build_configuration"}, {}, detail)) return false;
        std::uint32_t app_id = 0U;
        std::string platform, architecture, configuration;
        if (!u32(*target, "steam_app_id", app_id, detail) || app_id == 0U ||
            !json::get_u64(*target, "steam_build_id", staged.steam_build_id, detail) ||
            !u32(*target, "depot_id", staged.depot_id, detail) ||
            !json::get_u64(*target, "depot_manifest_gid", staged.depot_manifest_gid, detail) ||
            !json::get_string(*target, "platform", platform, detail) || platform != "windows" ||
            !json::get_string(*target, "architecture", architecture, detail) || architecture != "x86_64" ||
            !json::get_string(*target, "build_configuration", configuration, detail) || configuration != "shipping" ||
            staged.steam_build_id == 0U || staged.depot_id == 0U || staged.depot_manifest_gid == 0U) {
            if (detail.empty()) detail = "compiler target identity is unsupported or incomplete";
            return false;
        }

        const value* oracle = nullptr;
        if (!json::get_object(root, "oracle", oracle, detail) ||
            !json::require_object_keys(*oracle,
                {"executable", "binds_cache", "shipping_cache", "depot_manifest", "pe_codeview"}, {}, detail)) return false;
        for (const auto name : {"executable", "binds_cache", "shipping_cache", "depot_manifest"}) {
            const value* seal = nullptr;
            std::uint64_t byte_len = 0U;
            sha256_digest digest{};
            if (!json::get_object(*oracle, name, seal, detail) ||
                !json::require_object_keys(*seal, {"byte_len", "sha256"}, {"steam_content_sha1"}, detail) ||
                !json::get_u64(*seal, "byte_len", byte_len, detail) || byte_len == 0U ||
                !digest_field(*seal, "sha256", digest, detail)) return false;
            if (std::string_view(name) != "depot_manifest") {
                const value* steam_sha = seal->find("steam_content_sha1");
                if (steam_sha == nullptr || steam_sha->kind != value_kind::string ||
                    steam_sha->text.size() != 40U ||
                    !std::all_of(steam_sha->text.begin(), steam_sha->text.end(), [](const char ch) {
                        return (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f');
                    })) {
                    detail = std::string(name) + " has no valid Steam SHA-1 seal";
                    return false;
                }
            }
            if (std::string_view(name) == "binds_cache") {
                staged.oracle_binds_cache.byte_len = byte_len;
                staged.oracle_binds_cache.sha256 = digest;
            } else if (std::string_view(name) == "shipping_cache") {
                staged.oracle_shipping_cache.byte_len = byte_len;
                staged.oracle_shipping_cache.sha256 = digest;
            }
        }
        const value* codeview = nullptr;
        std::string guid;
        std::uint32_t age = 0U;
        if (!json::get_object(*oracle, "pe_codeview", codeview, detail) ||
            !json::require_object_keys(*codeview, {"guid", "age"}, {}, detail) ||
            !json::get_string(*codeview, "guid", guid, detail) || guid.empty() ||
            !u32(*codeview, "age", age, detail)) return false;

        const value* binds = nullptr;
        if (!json::get_object(root, "binds", binds, detail) ||
            !json::require_object_keys(*binds,
                {"wire_schema_version", "struct_count", "class_count", "method_count",
                 "struct_property_count", "class_property_count", "canonical_database_sha256"}, {}, detail)) return false;
        std::uint32_t binds_version = 0U;
        sha256_digest binds_digest{};
        for (const auto name : {"struct_count", "class_count", "method_count", "struct_property_count", "class_property_count"}) {
            std::uint64_t count = 0U;
            if (!json::get_u64(*binds, name, count, detail) || count == 0U) return false;
        }
        if (!u32(*binds, "wire_schema_version", binds_version, detail) || binds_version == 0U ||
            !digest_field(*binds, "canonical_database_sha256", binds_digest, detail)) return false;

        const value* engine = nullptr;
        if (!json::get_object(root, "engine", engine, detail) ||
            !json::require_object_keys(*engine,
                {"as_create_version", "ordered_engine_properties", "registration_trace",
                 "registration_trace_count", "post_bind_snapshot"}, {}, detail) ||
            !u32(*engine, "as_create_version", staged.as_create_version, detail) ||
            staged.as_create_version == 0U ||
            !json::get_u64(*engine, "registration_trace_count", staged.registration_trace_count, detail) ||
            staged.registration_trace_count == 0U ||
            !find_blob(*engine, "ordered_engine_properties", staged.ordered_engine_properties, staged.all_blobs, detail) ||
            !find_blob(*engine, "registration_trace", staged.registration_trace, staged.all_blobs, detail) ||
            !find_blob(*engine, "post_bind_snapshot", staged.post_bind_snapshot, staged.all_blobs, detail)) return false;

        const value* unreal = nullptr;
        std::uint32_t metadata_version = 0U;
        if (!json::get_object(root, "unreal_semantics", unreal, detail) ||
            !json::require_object_keys(*unreal, {"reflected_type_graph", "metadata_schema_version"}, {}, detail) ||
            !add_blob(*unreal, "reflected_type_graph", staged.all_blobs, detail) ||
            !u32(*unreal, "metadata_schema_version", metadata_version, detail) || metadata_version == 0U) return false;

        const value* frontend = nullptr;
        if (!json::get_object(root, "frontend", frontend, detail) ||
            !json::require_object_keys(*frontend,
                {"preprocessor_config", "class_generator_config", "compiler_options"}, {}, detail) ||
            !find_blob(*frontend, "preprocessor_config", staged.preprocessor_config, staged.all_blobs, detail) ||
            !find_blob(*frontend, "class_generator_config", staged.class_generator_config, staged.all_blobs, detail) ||
            !find_blob(*frontend, "compiler_options", staged.compiler_options, staged.all_blobs, detail)) return false;

        const value* bytecode = nullptr;
        std::string opcode_version;
        if (!json::get_object(root, "bytecode", bytecode, detail) ||
            !json::require_object_keys(*bytecode,
                {"opcode_table_version", "opcode_table", "operand_schema", "codegen_probe_corpus",
                 "expected_probe_results"}, {}, detail) ||
            !json::get_string(*bytecode, "opcode_table_version", opcode_version, detail) || opcode_version.empty() ||
            !add_blob(*bytecode, "opcode_table", staged.all_blobs, detail) ||
            !add_blob(*bytecode, "operand_schema", staged.all_blobs, detail) ||
            !add_blob(*bytecode, "codegen_probe_corpus", staged.all_blobs, detail) ||
            !add_blob(*bytecode, "expected_probe_results", staged.all_blobs, detail)) return false;

        const value* writer = nullptr;
        std::uint32_t writer_version = 0U;
        std::uint32_t build_id = 0U;
        if (!json::get_object(root, "cache_writer", writer, detail) ||
            !json::require_object_keys(*writer,
                {"format_version", "serializer_schema", "build_identifier", "reference_table_order",
                 "normalized_oracle_corpus"}, {}, detail) ||
            !u32(*writer, "format_version", writer_version, detail) || writer_version == 0U ||
            !u32(*writer, "build_identifier", build_id, detail) ||
            !add_blob(*writer, "serializer_schema", staged.all_blobs, detail) ||
            !add_blob(*writer, "reference_table_order", staged.all_blobs, detail) ||
            !add_blob(*writer, "normalized_oracle_corpus", staged.all_blobs, detail)) return false;
        std::memcpy(&staged.build_identifier, &build_id, sizeof(build_id));

        const value* qualification = nullptr;
        bool qualified = false;
        if (!json::get_object(root, "qualification", qualification, detail) ||
            !json::require_object_keys(*qualification,
                {"required_probe_suite_version", "diagnostic_parity", "semantic_parity", "qualified"}, {}, detail) ||
            !json::get_string(*qualification, "required_probe_suite_version", staged.required_probe_suite_version, detail) ||
            staged.required_probe_suite_version.empty() ||
            !add_blob(*qualification, "diagnostic_parity", staged.all_blobs, detail) ||
            !add_blob(*qualification, "semantic_parity", staged.all_blobs, detail) ||
            !json::get_bool(*qualification, "qualified", qualified, detail) || !qualified) {
            if (detail.empty()) detail = "compiler profile is not qualified";
            return false;
        }

        std::map<std::string, std::pair<std::uint64_t, sha256_digest>> paths;
        for (const auto& blob : staged.all_blobs) {
            std::string folded = blob.path;
            std::transform(folded.begin(), folded.end(), folded.begin(), [](const unsigned char ch) {
                return static_cast<char>(std::tolower(ch));
            });
            const auto [iterator, inserted] = paths.emplace(
                std::move(folded), std::make_pair(blob.byte_len, blob.sha256));
            if (!inserted && iterator->second != std::make_pair(blob.byte_len, blob.sha256)) {
                detail = "compiler profile contains conflicting colliding blob paths";
                return false;
            }
        }
        output = std::move(staged);
        detail.clear();
        return true;
    } catch (const std::exception& exception) {
        detail = exception.what();
        return false;
    } catch (...) {
        detail = "unknown compiler manifest conversion failure";
        return false;
    }
}

bool parse_registry_profile_payloads(
    const std::string_view ordered_properties_json,
    const std::string_view registration_trace_json,
    const std::string_view post_bind_snapshot_json,
    const std::uint64_t expected_trace_count,
    registry_profile& output,
    std::string& detail) {
    try {
        value properties_root, trace_root, snapshot_root;
        if (!parsed(ordered_properties_json, properties_root, detail) ||
            !parsed(registration_trace_json, trace_root, detail) ||
            !parsed(post_bind_snapshot_json, snapshot_root, detail)) return false;
        registry_profile staged;
        sha256_digest properties_identity{};
        sha256_digest trace_identity{};
        if (!parse_properties(properties_root, staged, properties_identity, detail) ||
            !parse_trace(trace_root, staged, trace_identity, detail) ||
            staged.registrations.size() != expected_trace_count ||
            !parse_snapshot(snapshot_root, staged, properties_identity, trace_identity, detail)) {
            if (staged.registrations.size() != expected_trace_count && detail.empty()) {
                detail = "registration trace count does not match the compiler manifest";
            }
            return false;
        }
        output = std::move(staged);
        detail.clear();
        return true;
    } catch (const std::exception& exception) {
        detail = exception.what();
        return false;
    } catch (...) {
        detail = "unknown registry profile conversion failure";
        return false;
    }
}

bool parse_frontend_profile_payloads(
    const std::string_view preprocessor_json,
    const std::string_view class_generator_json,
    const std::string_view compiler_options_json,
    preprocessor_options& preprocessor,
    compiler_options& compiler,
    std::string& detail) {
    try {
        value pre_root, class_root, options_root;
        if (!parsed(preprocessor_json, pre_root, detail) ||
            !parsed(class_generator_json, class_root, detail) ||
            !parsed(compiler_options_json, options_root, detail)) return false;
        if (!json::require_object_keys(pre_root,
                {"schema", "schema_version", "automatic_imports", "warn_on_manual_import_statements",
                 "use_editor_scripts", "effective_flags", "default_function_blueprint_callable",
                 "default_property_edit_specifier", "default_property_edit_specifier_for_structs",
                 "default_property_blueprint_specifier", "static_class_mode", "script_float_is_float64",
                 "angelscript_haze", "enforce_server_rpc_validation",
                 "blueprint_event_argument_specializations", "native_super_types", "canonical_sha256"}, {}, detail) ||
            !exact_schema(pre_root, "gore.as.preprocessor-config", detail)) return false;
        sha256_digest ignored{};
        bool ignored_bool = false;
        preprocessor_options staged_pre;
        if (!digest_field(pre_root, "canonical_sha256", ignored, detail) ||
            !json::get_bool(pre_root, "automatic_imports", staged_pre.automatic_imports, detail) ||
            !json::get_bool(pre_root, "warn_on_manual_import_statements", ignored_bool, detail) ||
            !json::get_bool(pre_root, "use_editor_scripts", ignored_bool, detail) ||
            !json::get_bool(pre_root, "default_function_blueprint_callable", staged_pre.default_function_blueprint_callable, detail) ||
            !json::get_bool(pre_root, "script_float_is_float64", staged_pre.script_float_is_float64, detail) ||
            !json::get_bool(pre_root, "angelscript_haze", staged_pre.angelscript_haze, detail) ||
            !json::get_bool(pre_root, "enforce_server_rpc_validation", staged_pre.enforce_server_rpc_validation, detail)) return false;
        std::string edit, struct_edit, blueprint, static_mode;
        if (!json::get_string(pre_root, "default_property_edit_specifier", edit, detail) ||
            !parse_enum_edit(edit, staged_pre.default_property_edit, detail) ||
            !json::get_string(pre_root, "default_property_edit_specifier_for_structs", struct_edit, detail) ||
            !parse_enum_edit(struct_edit, staged_pre.default_struct_property_edit, detail) ||
            !json::get_string(pre_root, "default_property_blueprint_specifier", blueprint, detail) ||
            !parse_enum_blueprint(blueprint, staged_pre.default_property_blueprint, detail) ||
            !json::get_string(pre_root, "static_class_mode", static_mode, detail) ||
            !parse_enum_static_class(static_mode, staged_pre.static_classes, detail)) return false;

        const value* flags = nullptr;
        if (!json::get_array(pre_root, "effective_flags", flags, detail) || flags->elements.size() > max_preprocessor_flags) return false;
        staged_pre.flags.reserve(flags->elements.size());
        for (const auto& item : flags->elements) {
            preprocessor_flag flag;
            std::uint32_t ordinal = 0U;
            if (!json::require_object_keys(item, {"ordinal", "name", "value"}, {}, detail) ||
                !u32(item, "ordinal", ordinal, detail) || ordinal != staged_pre.flags.size() ||
                !json::get_string(item, "name", flag.name, detail) || flag.name.empty() ||
                !json::get_bool(item, "value", flag.value, detail)) return false;
            staged_pre.flags.push_back(std::move(flag));
        }

        const value* specializations = nullptr;
        if (!json::get_array(pre_root, "blueprint_event_argument_specializations", specializations, detail) ||
            specializations->elements.size() > 4096U) return false;
        for (const auto& item : specializations->elements) {
            if (item.kind != value_kind::string || item.text.empty()) {
                detail = "blueprint event specialization is not a nonempty string";
                return false;
            }
            staged_pre.blueprint_event_argument_specializations.push_back(item.text);
        }

        const value* supers = nullptr;
        if (!json::get_array(pre_root, "native_super_types", supers, detail) || supers->elements.size() > 1'000'000U) return false;
        staged_pre.native_super_types.reserve(supers->elements.size());
        for (const auto& item : supers->elements) {
            native_super_type native;
            std::uint32_t ordinal = 0U;
            std::string kind;
            if (!json::require_object_keys(item,
                    {"ordinal", "angelscript_type_name", "unreal_class_path", "property_offset", "kind",
                     "cannot_derive_angelscript"}, {}, detail) ||
                !u32(item, "ordinal", ordinal, detail) || ordinal != staged_pre.native_super_types.size() ||
                !json::get_string(item, "angelscript_type_name", native.angelscript_type_name, detail) ||
                !json::get_string(item, "unreal_class_path", native.unreal_class_path, detail) ||
                !json::get_u64(item, "property_offset", native.property_offset, detail) ||
                !json::get_string(item, "kind", kind, detail) ||
                !parse_enum_native_super(kind, native.kind, detail) ||
                !json::get_bool(item, "cannot_derive_angelscript", native.cannot_derive_angelscript, detail)) return false;
            staged_pre.native_super_types.push_back(std::move(native));
        }

        compiler_options staged_compiler;
        if (!json::require_object_keys(class_root,
                {"schema", "schema_version", "mark_non_uproperty_properties_as_transient", "canonical_sha256"}, {}, detail) ||
            !exact_schema(class_root, "gore.as.class-generator-config", detail) ||
            !digest_field(class_root, "canonical_sha256", ignored, detail) ||
            !json::get_bool(class_root, "mark_non_uproperty_properties_as_transient",
                staged_compiler.mark_non_uproperty_properties_as_transient, detail)) return false;
        if (!json::require_object_keys(options_root,
                {"schema", "schema_version", "error_on_incorrect_editor_only_code",
                 "warn_on_divergent_comparison_operator_overloads", "warn_on_implicit_signed_unsigned_conversion",
                 "warn_on_increment_decrement_in_complex_expression", "warn_on_unused_return_value_for_const_methods",
                 "canonical_sha256"}, {}, detail) ||
            !exact_schema(options_root, "gore.as.compiler-options", detail) ||
            !digest_field(options_root, "canonical_sha256", ignored, detail) ||
            !json::get_bool(options_root, "error_on_incorrect_editor_only_code", staged_compiler.error_on_incorrect_editor_only_code, detail) ||
            !json::get_bool(options_root, "warn_on_divergent_comparison_operator_overloads",
                staged_compiler.warn_on_divergent_comparison_operator_overloads, detail) ||
            !json::get_bool(options_root, "warn_on_implicit_signed_unsigned_conversion",
                staged_compiler.warn_on_implicit_signed_unsigned_conversion, detail) ||
            !json::get_bool(options_root, "warn_on_increment_decrement_in_complex_expression",
                staged_compiler.warn_on_increment_decrement_in_complex_expression, detail) ||
            !json::get_bool(options_root, "warn_on_unused_return_value_for_const_methods",
                staged_compiler.warn_on_unused_return_value_for_const_methods, detail)) return false;

        preprocessor = std::move(staged_pre);
        compiler = staged_compiler;
        detail.clear();
        return true;
    } catch (const std::exception& exception) {
        detail = exception.what();
        return false;
    } catch (...) {
        detail = "unknown frontend profile conversion failure";
        return false;
    }
}

} // namespace gore::as::standalone
