#include "gore_as_standalone/registry_profile.hpp"

#include "as_callfunc.h"
#include "as_objecttype.h"
#include "as_property.h"
#include "as_scriptengine.h"
#include "as_scriptfunction.h"
#include "as_string.h"
#include "as_typeinfo.h"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <malloc.h>
#include <map>
#include <new>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>

namespace gore::as::standalone {
namespace {

constexpr std::size_t no_ordinal = static_cast<std::size_t>(-1);
constexpr std::size_t max_host_stubs = 1'000'000U;
constexpr std::size_t max_registrations = 2'000'000U;
constexpr std::size_t max_declaration_bytes = 64U * 1024U;
constexpr std::uint32_t max_object_bytes = 64U * 1024U * 1024U;
constexpr std::uint32_t max_offset = 256U * 1024U * 1024U;
constexpr std::uint32_t public_object_flag_mask = 0x003f'ffffU;
constexpr std::uint32_t function_trait_mask = 0x00ff'ffffU;

registry_replay_result fail(
    const registry_replay_phase phase,
    const std::size_t ordinal,
    std::string detail,
    const int code = asINVALID_CONFIGURATION) {
    return {code < 0 ? code : asERROR, phase, ordinal, std::move(detail)};
}

bool valid_alignment(const std::uint32_t value) noexcept {
    return value != 0U && value <= 4096U && (value & (value - 1U)) == 0U;
}

bool valid_text(const std::string& value) noexcept {
    if (value.empty() || value.size() > max_declaration_bytes ||
        value.find('\0') != std::string::npos) {
        return false;
    }
    return std::none_of(value.begin(), value.end(), [](const unsigned char character) {
        return character < 0x20U || character == 0x7fU;
    });
}

bool valid_namespace(const std::string& value) noexcept {
    if (value.empty()) return true;
    if (value.size() > max_declaration_bytes || value.find('\0') != std::string::npos ||
        value.compare(0U, 2U, "::") == 0 ||
        (value.size() >= 2U && value.compare(value.size() - 2U, 2U, "::") == 0)) {
        return false;
    }
    for (std::size_t index = 0U; index < value.size();) {
        const unsigned char character = static_cast<unsigned char>(value[index]);
        if (character == ':') {
            if (index + 1U >= value.size() || value[index + 1U] != ':') return false;
            index += 2U;
            continue;
        }
        if (!(character == '_' || character >= 0x80U ||
              (character >= '0' && character <= '9') ||
              (character >= 'A' && character <= 'Z') ||
              (character >= 'a' && character <= 'z'))) {
            return false;
        }
        ++index;
    }
    return value.find("::::") == std::string::npos;
}

bool valid_identifier(const std::string& value) noexcept {
    if (!valid_text(value)) return false;
    return std::all_of(value.begin(), value.end(), [](const unsigned char character) {
        return character == '_' || character >= 0x80U ||
            (character >= '0' && character <= '9') ||
            (character >= 'A' && character <= 'Z') ||
            (character >= 'a' && character <= 'z');
    });
}

bool valid_call_convention(const call_convention value) noexcept {
    return static_cast<unsigned>(value) <=
        static_cast<unsigned>(call_convention::thiscall_object_first);
}

bool valid_behaviour(const object_behaviour value) noexcept {
    return static_cast<unsigned>(value) <=
        static_cast<unsigned>(object_behaviour::release_refs);
}

bool valid_adapter(const template_validation_adapter value) noexcept {
    return static_cast<unsigned>(value) <=
        static_cast<unsigned>(template_validation_adapter::t_soft_class_ptr);
}

asEEngineProp to_engine_property(const engine_property property) {
    switch (property) {
    case engine_property::allow_unsafe_references: return asEP_ALLOW_UNSAFE_REFERENCES;
    case engine_property::optimize_bytecode: return asEP_OPTIMIZE_BYTECODE;
    case engine_property::copy_script_sections: return asEP_COPY_SCRIPT_SECTIONS;
    case engine_property::max_stack_size: return asEP_MAX_STACK_SIZE;
    case engine_property::use_character_literals: return asEP_USE_CHARACTER_LITERALS;
    case engine_property::allow_multiline_strings: return asEP_ALLOW_MULTILINE_STRINGS;
    case engine_property::allow_implicit_handle_types: return asEP_ALLOW_IMPLICIT_HANDLE_TYPES;
    case engine_property::build_without_line_cues: return asEP_BUILD_WITHOUT_LINE_CUES;
    case engine_property::init_global_vars_after_build: return asEP_INIT_GLOBAL_VARS_AFTER_BUILD;
    case engine_property::require_enum_scope: return asEP_REQUIRE_ENUM_SCOPE;
    case engine_property::script_scanner: return asEP_SCRIPT_SCANNER;
    case engine_property::include_jit_instructions: return asEP_INCLUDE_JIT_INSTRUCTIONS;
    case engine_property::string_encoding: return asEP_STRING_ENCODING;
    case engine_property::property_accessor_mode: return asEP_PROPERTY_ACCESSOR_MODE;
    case engine_property::expand_default_array_to_template: return asEP_EXPAND_DEF_ARRAY_TO_TMPL;
    case engine_property::auto_garbage_collect: return asEP_AUTO_GARBAGE_COLLECT;
    case engine_property::disallow_global_vars: return asEP_DISALLOW_GLOBAL_VARS;
    case engine_property::always_implement_default_construct: return asEP_ALWAYS_IMPL_DEFAULT_CONSTRUCT;
    case engine_property::compiler_warnings: return asEP_COMPILER_WARNINGS;
    case engine_property::disallow_value_assign_for_reference_type: return asEP_DISALLOW_VALUE_ASSIGN_FOR_REF_TYPE;
    case engine_property::alter_syntax_named_args: return asEP_ALTER_SYNTAX_NAMED_ARGS;
    case engine_property::disable_integer_division: return asEP_DISABLE_INTEGER_DIVISION;
    case engine_property::disallow_empty_list_elements: return asEP_DISALLOW_EMPTY_LIST_ELEMENTS;
    case engine_property::private_property_as_protected: return asEP_PRIVATE_PROP_AS_PROTECTED;
    case engine_property::allow_unicode_identifiers: return asEP_ALLOW_UNICODE_IDENTIFIERS;
    case engine_property::heredoc_trim_mode: return asEP_HEREDOC_TRIM_MODE;
    case engine_property::max_nested_calls: return asEP_MAX_NESTED_CALLS;
    case engine_property::generic_call_mode: return asEP_GENERIC_CALL_MODE;
    case engine_property::automatic_imports: return asEP_AUTOMATIC_IMPORTS;
    case engine_property::typecheck_switch_enums: return asEP_TYPECHECK_SWITCH_ENUMS;
    case engine_property::allow_double_type: return asEP_ALLOW_DOUBLE_TYPE;
    case engine_property::float_is_float64: return asEP_FLOAT_IS_FLOAT64;
    case engine_property::warn_on_float_constants_for_doubles: return asEP_WARN_ON_FLOAT_CONSTANTS_FOR_DOUBLES;
    case engine_property::warn_integer_division: return asEP_WARN_INTEGER_DIVISION;
    }
    return asEP_LAST_PROPERTY;
}

asDWORD to_call_convention(const call_convention convention) {
    switch (convention) {
    case call_convention::cdecl_call: return asCALL_CDECL;
    case call_convention::stdcall_call: return asCALL_STDCALL;
    case call_convention::thiscall_as_global: return asCALL_THISCALL_ASGLOBAL;
    case call_convention::thiscall: return asCALL_THISCALL;
    case call_convention::cdecl_object_last: return asCALL_CDECL_OBJLAST;
    case call_convention::cdecl_object_first: return asCALL_CDECL_OBJFIRST;
    case call_convention::generic: return asCALL_GENERIC;
    case call_convention::thiscall_object_last: return asCALL_THISCALL_OBJLAST;
    case call_convention::thiscall_object_first: return asCALL_THISCALL_OBJFIRST;
    }
    return std::numeric_limits<asDWORD>::max();
}

asEBehaviours to_behaviour(const object_behaviour behaviour) {
    switch (behaviour) {
    case object_behaviour::construct: return asBEHAVE_CONSTRUCT;
    case object_behaviour::list_construct: return asBEHAVE_LIST_CONSTRUCT;
    case object_behaviour::destruct: return asBEHAVE_DESTRUCT;
    case object_behaviour::factory: return asBEHAVE_FACTORY;
    case object_behaviour::list_factory: return asBEHAVE_LIST_FACTORY;
    case object_behaviour::add_ref: return asBEHAVE_ADDREF;
    case object_behaviour::release: return asBEHAVE_RELEASE;
    case object_behaviour::get_weakref_flag: return asBEHAVE_GET_WEAKREF_FLAG;
    case object_behaviour::template_callback: return asBEHAVE_TEMPLATE_CALLBACK;
    case object_behaviour::get_ref_count: return asBEHAVE_GETREFCOUNT;
    case object_behaviour::set_gc_flag: return asBEHAVE_SETGCFLAG;
    case object_behaviour::get_gc_flag: return asBEHAVE_GETGCFLAG;
    case object_behaviour::enum_refs: return asBEHAVE_ENUMREFS;
    case object_behaviour::release_refs: return asBEHAVE_RELEASEREFS;
    }
    return asBEHAVE_CONSTRUCT;
}

registration_result_kind result_kind_for(const registration_kind kind) {
    switch (kind) {
    case registration_kind::object_type: return registration_result_kind::object_type;
    case registration_kind::interface_type: return registration_result_kind::interface_type;
    case registration_kind::interface_method: return registration_result_kind::interface_method;
    case registration_kind::object_property: return registration_result_kind::object_property;
    case registration_kind::object_method: return registration_result_kind::object_method;
    case registration_kind::object_behaviour: return registration_result_kind::object_behaviour;
    case registration_kind::global_property: return registration_result_kind::global_property;
    case registration_kind::global_function: return registration_result_kind::global_function;
    case registration_kind::enum_type: return registration_result_kind::enum_type;
    case registration_kind::enum_value: return registration_result_kind::enum_value;
    case registration_kind::funcdef: return registration_result_kind::funcdef;
    case registration_kind::typedef_type: return registration_result_kind::typedef_type;
    case registration_kind::string_factory: return registration_result_kind::string_factory;
    case registration_kind::default_array_type: return registration_result_kind::default_array_type;
    }
    return registration_result_kind::object_type;
}

std::string callable_type_declaration(std::string declaration) {
    constexpr std::string_view marker = "class ";
    std::size_t position = 0U;
    while ((position = declaration.find(marker, position)) != std::string::npos) {
        declaration.erase(position, marker.size());
    }
    return declaration;
}

void inert_global_stub() {}
void inert_generic_stub(asIScriptGeneric*) {}

class inert_method_stub final {
public:
    void invoke() {}
};

asSFuncPtr callable_stub(const call_convention convention) {
    switch (convention) {
    case call_convention::generic:
        return asFUNCTION(inert_generic_stub);
    case call_convention::thiscall_as_global:
    case call_convention::thiscall:
    case call_convention::thiscall_object_last:
    case call_convention::thiscall_object_first:
        return asMETHOD(inert_method_stub, invoke);
    case call_convention::cdecl_call:
    case call_convention::stdcall_call:
    case call_convention::cdecl_object_last:
    case call_convention::cdecl_object_first:
        return asFUNCTION(inert_global_stub);
    }
    return asSFuncPtr{};
}

bool validate_class_template(asITypeInfo* type, asCString* error) {
    if (type->GetSubTypeCount() != 1U) {
        return false;
    }
    asITypeInfo* subtype = type->GetSubType(0U);
    if (subtype == nullptr || (subtype->GetFlags() & asOBJ_VALUE) != 0U) {
        if (error != nullptr) {
            *error = "Subtype must be a class type";
        }
        return false;
    }
    return true;
}

asSFuncPtr template_callback(const template_validation_adapter adapter) {
    switch (adapter) {
    case template_validation_adapter::t_subclass_of:
    case template_validation_adapter::t_object_ptr:
    case template_validation_adapter::t_weak_object_ptr:
    case template_validation_adapter::t_soft_object_ptr:
    case template_validation_adapter::t_soft_class_ptr:
        return asFUNCTION(validate_class_template);
    case template_validation_adapter::none:
    case template_validation_adapter::t_array:
    case template_validation_adapter::t_map:
    case template_validation_adapter::t_set:
    case template_validation_adapter::t_optional:
        return asSFuncPtr{};
    }
    return asSFuncPtr{};
}

asECompileOutType to_compile_out(const compile_out_mode value) {
    switch (value) {
    case compile_out_mode::compile_calls: return asECompileOutType::CompileCalls;
    case compile_out_mode::compile_out_entirely: return asECompileOutType::CompileOutEntirely;
    case compile_out_mode::replace_with_first_param: return asECompileOutType::ReplaceWithFirstParam;
    case compile_out_mode::compile_out_as_method_chain: return asECompileOutType::CompileOutAsMethodChain;
    }
    return asECompileOutType::CompileCalls;
}

asEFirstParamMetaData to_first_param(const first_param_metadata value) {
    switch (value) {
    case first_param_metadata::none: return asEFirstParamMetaData::None;
    case first_param_metadata::script_function: return asEFirstParamMetaData::ScriptFunction;
    case first_param_metadata::script_object_type: return asEFirstParamMetaData::ScriptObjectType;
    }
    return asEFirstParamMetaData::None;
}

class string_pool final : public asIStringFactory {
public:
    const void* GetStringConstant(const char* data, const asUINT length) override {
        const std::string key(data, static_cast<std::size_t>(length));
        auto [iterator, inserted] = values_.try_emplace(key, record{key, 0U});
        (void)inserted;
        ++iterator->second.references;
        return &iterator->second;
    }

    int ReleaseStringConstant(const void* value) override {
        if (value == nullptr) {
            return asINVALID_ARG;
        }
        const auto* record_pointer = static_cast<const record*>(value);
        const auto iterator = values_.find(record_pointer->bytes);
        if (iterator == values_.end() || &iterator->second != record_pointer ||
            iterator->second.references == 0U) {
            return asINVALID_ARG;
        }
        --iterator->second.references;
        if (iterator->second.references == 0U) {
            values_.erase(iterator);
        }
        return asSUCCESS;
    }

    int GetRawStringData(const void* value, char* data, asUINT* length) const override {
        if (value == nullptr || length == nullptr) {
            return asINVALID_ARG;
        }
        const auto* record_pointer = static_cast<const record*>(value);
        const auto iterator = values_.find(record_pointer->bytes);
        if (iterator == values_.end() || &iterator->second != record_pointer ||
            iterator->second.bytes.size() > std::numeric_limits<asUINT>::max()) {
            return asINVALID_ARG;
        }
        const auto actual_length = static_cast<asUINT>(iterator->second.bytes.size());
        if (data != nullptr) {
            std::memcpy(data, iterator->second.bytes.data(), actual_length);
        }
        *length = actual_length;
        return asSUCCESS;
    }

private:
    struct record {
        std::string bytes;
        std::size_t references;
    };
    std::map<std::string, record> values_;
};

struct aligned_delete {
    void operator()(void* pointer) const noexcept { _aligned_free(pointer); }
};
using aligned_pointer = std::unique_ptr<void, aligned_delete>;

struct replay_maps {
    std::unordered_map<std::uint32_t, asCTypeInfo*> types;
    std::unordered_map<std::uint32_t, asCObjectProperty*> properties;
    std::unordered_map<std::uint32_t, asCScriptFunction*> functions;
    std::unordered_map<std::uint32_t, asCGlobalProperty*> globals;
    std::unordered_map<std::uint32_t, std::string> type_declarations;
};

const host_stub* find_stub(const registry_profile& profile, const std::uint32_t id) {
    if (id >= profile.host_stubs.size() || profile.host_stubs[id].stub_id != id) {
        return nullptr;
    }
    return &profile.host_stubs[id];
}

registry_replay_result validate_profile(const registry_profile& profile) {
    if (profile.engine_properties.size() > 4096U ||
        profile.host_stubs.size() > max_host_stubs ||
        profile.registrations.empty() ||
        profile.registrations.size() > max_registrations ||
        profile.expected_results.size() != profile.registrations.size() ||
        profile.final_states.size() > max_registrations) {
        return fail(registry_replay_phase::validate_profile, no_ordinal, "registry profile count is invalid");
    }
    for (std::size_t index = 0U; index < profile.engine_properties.size(); ++index) {
        if (profile.engine_properties[index].ordinal != index ||
            to_engine_property(profile.engine_properties[index].property) == asEP_LAST_PROPERTY) {
            return fail(registry_replay_phase::validate_profile, index, "engine property order or identity is invalid");
        }
    }
    for (std::size_t index = 0U; index < profile.host_stubs.size(); ++index) {
        const host_stub& stub = profile.host_stubs[index];
        if (stub.stub_id != index ||
            static_cast<unsigned>(stub.kind) > static_cast<unsigned>(host_stub_kind::object)) {
            return fail(registry_replay_phase::validate_profile, index, "host stub order is invalid");
        }
        if ((stub.kind == host_stub_kind::storage &&
             (stub.byte_len > max_object_bytes || !valid_alignment(stub.alignment))) ||
            (stub.kind != host_stub_kind::storage &&
             (stub.byte_len != 0U || stub.alignment != 1U))) {
            return fail(registry_replay_phase::validate_profile, index, "host stub descriptor is invalid");
        }
    }

    std::unordered_set<std::uint32_t> type_ids;
    std::unordered_set<std::uint32_t> object_types;
    std::unordered_set<std::uint32_t> interface_types;
    std::unordered_set<std::uint32_t> enum_types;
    std::unordered_set<std::uint32_t> function_ids;
    std::unordered_set<std::uint32_t> property_ids;
    std::unordered_set<std::uint32_t> used_stubs;
    std::unordered_map<std::uint32_t, std::uint32_t> type_results;
    std::unordered_set<std::uint32_t> engine_type_ids;
    std::unordered_set<std::uint32_t> engine_function_ids;
    std::unordered_set<std::uint32_t> global_property_indices;
    std::unordered_set<std::uint64_t> member_indices;
    std::unordered_set<std::uint32_t> expected_final_types;
    std::unordered_set<std::uint32_t> expected_final_properties;
    std::unordered_set<std::uint32_t> expected_final_functions;
    std::unordered_set<std::uint32_t> expected_final_globals;
    bool string_factory_seen = false;
    bool default_array_seen = false;
    for (std::size_t index = 0U; index < profile.registrations.size(); ++index) {
        const registration_entry& entry = profile.registrations[index];
        const registration_result& expected = profile.expected_results[index];
        if (entry.ordinal != index || entry.registration_id != index ||
            static_cast<unsigned>(entry.kind) >
                static_cast<unsigned>(registration_kind::default_array_type) ||
            expected.ordinal != index || expected.trace_registration_id != index ||
            expected.kind != result_kind_for(entry.kind)) {
            return fail(registry_replay_phase::validate_profile, index, "registration/result order is invalid");
        }
        if (!valid_namespace(entry.context.name_space) ||
            (entry.context.config_group.has_value() &&
             !valid_text(*entry.context.config_group))) {
            return fail(registry_replay_phase::validate_profile, index, "registration context is invalid");
        }

        const auto require_stub = [&](const std::uint32_t id, const host_stub_kind kind) {
            const host_stub* stub = find_stub(profile, id);
            if (stub == nullptr || stub->kind != kind) return false;
            used_stubs.insert(id);
            return true;
        };
        const auto require_object = [&]() {
            return object_types.count(entry.owner_type_id) != 0U;
        };
        switch (entry.kind) {
        case registration_kind::object_type:
            if (!valid_text(entry.declaration) || entry.byte_size > max_object_bytes ||
                !valid_alignment(entry.alignment) ||
                (entry.flags & ~public_object_flag_mask) != 0U ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object type descriptor is invalid");
            }
            object_types.insert(entry.logical_id);
            expected_final_types.insert(entry.logical_id);
            break;
        case registration_kind::interface_type:
            if (!valid_text(entry.declaration) || !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "interface descriptor is invalid");
            }
            interface_types.insert(entry.logical_id);
            expected_final_types.insert(entry.logical_id);
            break;
        case registration_kind::interface_method:
            if (interface_types.count(entry.owner_type_id) == 0U ||
                !valid_text(entry.declaration) ||
                !function_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "interface method descriptor is invalid");
            }
            expected_final_functions.insert(entry.logical_id);
            break;
        case registration_kind::object_property:
            if (!require_object() || !valid_text(entry.declaration) ||
                entry.byte_offset > max_offset || entry.composite_offset > max_offset ||
                entry.accessor_type > 255U || !property_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object property descriptor is invalid");
            }
            expected_final_properties.insert(entry.logical_id);
            break;
        case registration_kind::object_method:
            if (!require_object() || !valid_text(entry.declaration) ||
                !valid_call_convention(entry.convention) ||
                !require_stub(entry.callable_stub_id, host_stub_kind::callable) ||
                (entry.auxiliary_object_stub_id.has_value() &&
                 !require_stub(*entry.auxiliary_object_stub_id, host_stub_kind::object)) ||
                entry.validation_adapter != template_validation_adapter::none ||
                entry.composite_offset > max_offset || entry.accessor_type > 255U ||
                !function_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object method descriptor is invalid");
            }
            expected_final_functions.insert(entry.logical_id);
            break;
        case registration_kind::object_behaviour: {
            const bool is_template = entry.behaviour == object_behaviour::template_callback;
            const bool has_adapter = entry.validation_adapter != template_validation_adapter::none;
            if (!require_object() || !valid_text(entry.declaration) ||
                !valid_call_convention(entry.convention) || !valid_behaviour(entry.behaviour) ||
                !valid_adapter(entry.validation_adapter) ||
                !require_stub(entry.callable_stub_id, host_stub_kind::callable) ||
                (entry.auxiliary_object_stub_id.has_value() &&
                 !require_stub(*entry.auxiliary_object_stub_id, host_stub_kind::object)) ||
                is_template != has_adapter || entry.composite_offset > max_offset ||
                !function_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object behaviour descriptor is invalid");
            }
            if (has_adapter && template_callback(entry.validation_adapter).ptr.f.func == nullptr) {
                return fail(
                    registry_replay_phase::validate_profile, index,
                    "container template validator requires the still-unported G1R type-operations layer",
                    asNOT_SUPPORTED);
            }
            expected_final_functions.insert(entry.logical_id);
            break;
        }
        case registration_kind::global_property:
            if (!valid_text(entry.declaration) ||
                !require_stub(entry.storage_stub_id, host_stub_kind::storage) ||
                !property_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "global property descriptor is invalid");
            }
            expected_final_globals.insert(entry.logical_id);
            break;
        case registration_kind::global_function:
            if (!valid_text(entry.declaration) || !valid_call_convention(entry.convention) ||
                !require_stub(entry.callable_stub_id, host_stub_kind::callable) ||
                (entry.auxiliary_object_stub_id.has_value() &&
                 !require_stub(*entry.auxiliary_object_stub_id, host_stub_kind::object)) ||
                !function_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "global function descriptor is invalid");
            }
            expected_final_functions.insert(entry.logical_id);
            break;
        case registration_kind::enum_type:
            if (!valid_text(entry.declaration) || !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "enum descriptor is invalid");
            }
            enum_types.insert(entry.logical_id);
            break;
        case registration_kind::enum_value:
            if (enum_types.count(entry.owner_type_id) == 0U || !valid_identifier(entry.name)) {
                return fail(registry_replay_phase::validate_profile, index, "enum value descriptor is invalid");
            }
            break;
        case registration_kind::funcdef:
            if (!valid_text(entry.declaration) || !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "funcdef descriptor is invalid");
            }
            break;
        case registration_kind::typedef_type:
            if (!valid_identifier(entry.name) || !valid_text(entry.target_declaration) ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "typedef descriptor is invalid");
            }
            break;
        case registration_kind::string_factory:
            if (string_factory_seen || !valid_text(entry.declaration) ||
                !require_stub(entry.factory_object_stub_id, host_stub_kind::object)) {
                return fail(registry_replay_phase::validate_profile, index, "string factory descriptor is invalid");
            }
            string_factory_seen = true;
            break;
        case registration_kind::default_array_type:
            if (default_array_seen || !valid_text(entry.declaration)) {
                return fail(registry_replay_phase::validate_profile, index, "default array descriptor is invalid");
            }
            default_array_seen = true;
            break;
        }

        const auto owner_result_matches = [&]() {
            const auto iterator = type_results.find(entry.owner_type_id);
            return iterator != type_results.end() &&
                iterator->second == expected.owner_engine_type_id;
        };
        const auto unique_member = [&]() {
            const std::uint64_t key =
                (static_cast<std::uint64_t>(expected.owner_engine_type_id) << 32U) |
                expected.index;
            return member_indices.insert(key).second;
        };
        bool result_valid = false;
        switch (entry.kind) {
        case registration_kind::object_type:
        case registration_kind::interface_type:
        case registration_kind::enum_type:
        case registration_kind::funcdef:
            result_valid = expected.owner_engine_type_id == 0U && expected.index == 0U &&
                !expected.installed && engine_type_ids.insert(expected.engine_id).second;
            if (result_valid) type_results.emplace(entry.logical_id, expected.engine_id);
            break;
        case registration_kind::typedef_type:
            // RegisterTypedef returns the aliased primitive type id. Different
            // typedef declarations can therefore share this engine id.
            result_valid = expected.owner_engine_type_id == 0U && expected.index == 0U &&
                !expected.installed;
            if (result_valid) type_results.emplace(entry.logical_id, expected.engine_id);
            break;
        case registration_kind::interface_method:
        case registration_kind::object_method:
        case registration_kind::object_behaviour:
            result_valid = owner_result_matches() && expected.index == 0U &&
                !expected.installed && engine_function_ids.insert(expected.engine_id).second;
            break;
        case registration_kind::object_property:
        case registration_kind::enum_value:
            result_valid = expected.engine_id == 0U && owner_result_matches() &&
                !expected.installed && unique_member();
            break;
        case registration_kind::global_property:
            result_valid = expected.engine_id == 0U && expected.owner_engine_type_id == 0U &&
                !expected.installed && global_property_indices.insert(expected.index).second;
            break;
        case registration_kind::global_function:
            result_valid = expected.owner_engine_type_id == 0U && expected.index == 0U &&
                !expected.installed && engine_function_ids.insert(expected.engine_id).second;
            break;
        case registration_kind::string_factory:
        case registration_kind::default_array_type:
            result_valid = expected.engine_id == 0U && expected.owner_engine_type_id == 0U &&
                expected.index == 0U && expected.installed;
            break;
        }
        if (!result_valid) {
            return fail(registry_replay_phase::validate_profile, index, "captured registration result identity is invalid");
        }
    }

    if (used_stubs.size() != profile.host_stubs.size()) {
        return fail(registry_replay_phase::validate_profile, no_ordinal, "host stub table contains an unreferenced descriptor");
    }

    std::unordered_set<std::uint32_t> actual_final_types;
    std::unordered_set<std::uint32_t> actual_final_properties;
    std::unordered_set<std::uint32_t> actual_final_functions;
    std::unordered_set<std::uint32_t> actual_final_globals;
    for (std::size_t index = 0U; index < profile.final_states.size(); ++index) {
        const post_bind_state& state = profile.final_states[index];
        if (static_cast<unsigned>(state.kind) >
            static_cast<unsigned>(post_bind_state_kind::global_property)) {
            return fail(registry_replay_phase::validate_profile, index, "post-bind state kind is invalid");
        }
        switch (state.kind) {
        case post_bind_state_kind::object_type: {
            std::unordered_set<std::uint32_t> seen_interfaces;
            bool references_valid =
                state.interface_type_ids.size() == state.interface_vft_offsets.size();
            for (const std::uint32_t interface_id : state.interface_type_ids) {
                references_valid = references_valid &&
                    interface_types.count(interface_id) != 0U &&
                    seen_interfaces.insert(interface_id).second;
            }
            for (const std::optional<std::uint32_t> reference :
                 {state.base_type_id, state.shadow_type_id}) {
                references_valid = references_valid &&
                    (!reference.has_value() || expected_final_types.count(*reference) != 0U);
            }
            if (state.byte_size > max_object_bytes || !valid_alignment(state.alignment) ||
                !references_valid || expected_final_types.count(state.logical_id) == 0U ||
                !actual_final_types.insert(state.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object type final state is invalid");
            }
            break;
        }
        case post_bind_state_kind::object_property:
            if (state.byte_offset > max_offset || state.composite_offset > max_offset ||
                state.exposed_type > 255U ||
                expected_final_properties.count(state.logical_id) == 0U ||
                !actual_final_properties.insert(state.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object property final state is invalid");
            }
            break;
        case post_bind_state_kind::function:
            if ((state.trait_bits & ~function_trait_mask) != 0U ||
                state.exposed_type > 255U ||
                state.hidden_argument_index.has_value() != state.hidden_argument_default.has_value() ||
                (state.hidden_argument_default.has_value() &&
                 !valid_text(*state.hidden_argument_default)) ||
                static_cast<unsigned>(state.compile_out) >
                    static_cast<unsigned>(compile_out_mode::compile_out_as_method_chain) ||
                static_cast<unsigned>(state.first_param) >
                    static_cast<unsigned>(first_param_metadata::script_object_type) ||
                expected_final_functions.count(state.logical_id) == 0U ||
                !actual_final_functions.insert(state.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "function final state is invalid");
            }
            break;
        case post_bind_state_kind::global_property:
            if (state.is_pure_constant != state.pure_constant_value.has_value() ||
                expected_final_globals.count(state.logical_id) == 0U ||
                !actual_final_globals.insert(state.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "global property final state is invalid");
            }
            break;
        }
    }
    if (actual_final_types != expected_final_types ||
        actual_final_properties != expected_final_properties ||
        actual_final_functions != expected_final_functions ||
        actual_final_globals != expected_final_globals) {
        return fail(registry_replay_phase::validate_profile, no_ordinal, "post-bind final state does not cover every replayed identity exactly once");
    }
    return {};
}

asCObjectType* object_type(replay_maps& maps, const std::uint32_t logical_id) {
    const auto iterator = maps.types.find(logical_id);
    return iterator == maps.types.end() ? nullptr : CastToObjectType(iterator->second);
}

void* auxiliary_pointer(class registry_runtime::impl& runtime, std::uint32_t id);

registry_replay_result apply_context(
    asIScriptEngine& engine,
    const registration_entry& entry) {
    int code = engine.SetDefaultNamespace(entry.context.name_space.c_str());
    if (code < 0) {
        return fail(registry_replay_phase::apply_registration_context, entry.ordinal, "SetDefaultNamespace rejected captured context", code);
    }
    engine.SetDefaultAccessMask(entry.context.access_mask);
    if (entry.context.config_group.has_value()) {
        code = engine.BeginConfigGroup(entry.context.config_group->c_str());
        if (code < 0) {
            return fail(registry_replay_phase::apply_registration_context, entry.ordinal, "BeginConfigGroup rejected captured context", code);
        }
        code = engine.EndConfigGroup();
        if (code < 0) {
            return fail(registry_replay_phase::apply_registration_context, entry.ordinal, "EndConfigGroup rejected captured context", code);
        }
    }
    return {};
}

bool same_result(const registration_result& actual, const registration_result& expected) {
    return actual.kind == expected.kind && actual.engine_id == expected.engine_id &&
        actual.owner_engine_type_id == expected.owner_engine_type_id &&
        actual.index == expected.index && actual.installed == expected.installed;
}

} // namespace

class registry_runtime::impl final {
public:
    bool bound = false;
    std::unordered_map<std::uint32_t, aligned_pointer> storage;
    std::unordered_map<std::uint32_t, std::unique_ptr<std::byte>> objects;
    std::unique_ptr<string_pool> strings;
};

namespace {

void* auxiliary_pointer(registry_runtime::impl& runtime, const std::uint32_t id) {
    const auto iterator = runtime.objects.find(id);
    return iterator == runtime.objects.end() ? nullptr : iterator->second.get();
}

registry_replay_result register_one(
    asCScriptEngine& engine,
    const registry_profile& profile,
    const registration_entry& entry,
    registry_runtime::impl& runtime,
    replay_maps& maps,
    registration_result& actual) {
    actual.ordinal = entry.ordinal;
    actual.trace_registration_id = entry.registration_id;
    actual.kind = result_kind_for(entry.kind);
    int code = asERROR;
    asCObjectType* owner = object_type(maps, entry.owner_type_id);
    const auto owner_decl = [&]() -> const char* {
        const auto iterator = maps.type_declarations.find(entry.owner_type_id);
        return iterator == maps.type_declarations.end() ? nullptr : iterator->second.c_str();
    };
    const auto auxiliary = [&]() -> void* {
        return entry.auxiliary_object_stub_id.has_value()
            ? auxiliary_pointer(runtime, *entry.auxiliary_object_stub_id)
            : nullptr;
    };

    switch (entry.kind) {
    case registration_kind::object_type: {
        const asUINT before = engine.GetObjectTypeCount();
        code = engine.RegisterObjectType(
            entry.declaration.c_str(), static_cast<int>(entry.byte_size), entry.flags);
        if (code >= 0 && engine.GetObjectTypeCount() == before + 1U) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetObjectTypeByIndex(before));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered object type is not reflectable");
            type->alignment = static_cast<int>(entry.alignment);
            maps.types.emplace(entry.logical_id, type);
            maps.type_declarations.emplace(
                entry.logical_id, callable_type_declaration(entry.declaration));
            actual.engine_id = static_cast<std::uint32_t>(type->GetTypeId());
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "object type count did not advance exactly once");
        }
        break;
    }
    case registration_kind::interface_type: {
        const asUINT before = engine.GetObjectTypeCount();
        code = engine.RegisterInterface(entry.declaration.c_str());
        if (code >= 0 && engine.GetObjectTypeCount() == before + 1U) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetObjectTypeByIndex(before));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered interface is not reflectable");
            maps.types.emplace(entry.logical_id, type);
            maps.type_declarations.emplace(entry.logical_id, entry.declaration);
            actual.engine_id = static_cast<std::uint32_t>(type->GetTypeId());
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "interface count did not advance exactly once");
        }
        break;
    }
    case registration_kind::interface_method:
        code = engine.RegisterInterfaceMethod(owner_decl(), entry.declaration.c_str());
        if (code >= 0) {
            auto* function = static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered interface method is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, function);
            actual.engine_id = static_cast<std::uint32_t>(code);
            actual.owner_engine_type_id = static_cast<std::uint32_t>(owner->GetTypeId());
        }
        break;
    case registration_kind::object_property: {
        const asUINT before = owner->GetPropertyCount();
        code = engine.RegisterObjectProperty(
            owner_decl(), entry.declaration.c_str(), static_cast<int>(entry.byte_offset),
            static_cast<int>(entry.composite_offset), entry.is_composite_indirect,
            entry.accessor_type, entry.is_protected);
        if (code >= 0 && owner->GetPropertyCount() == before + 1U) {
            maps.properties.emplace(entry.logical_id, owner->properties[before]);
            actual.owner_engine_type_id = static_cast<std::uint32_t>(owner->GetTypeId());
            actual.index = before;
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "object property count did not advance exactly once");
        }
        break;
    }
    case registration_kind::object_method:
        code = engine.RegisterObjectMethod(
            owner_decl(), entry.declaration.c_str(), callable_stub(entry.convention),
            to_call_convention(entry.convention), nullptr, auxiliary(),
            static_cast<int>(entry.composite_offset), entry.is_composite_indirect,
            entry.accessor_type);
        if (code >= 0) {
            auto* function = static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered object method is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, function);
            actual.engine_id = static_cast<std::uint32_t>(code);
            actual.owner_engine_type_id = static_cast<std::uint32_t>(owner->GetTypeId());
        }
        break;
    case registration_kind::object_behaviour: {
        const asSFuncPtr function = entry.behaviour == object_behaviour::template_callback
            ? template_callback(entry.validation_adapter)
            : callable_stub(entry.convention);
        code = engine.RegisterObjectBehaviour(
            owner_decl(), to_behaviour(entry.behaviour), entry.declaration.c_str(), function,
            to_call_convention(entry.convention), nullptr, auxiliary(),
            static_cast<int>(entry.composite_offset), entry.is_composite_indirect);
        if (code >= 0) {
            auto* reflected_function =
                static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (reflected_function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered object behaviour is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, reflected_function);
            actual.engine_id = static_cast<std::uint32_t>(code);
            actual.owner_engine_type_id = static_cast<std::uint32_t>(owner->GetTypeId());
        }
        break;
    }
    case registration_kind::global_property: {
        const asUINT before = engine.GetGlobalPropertyCount();
        void* pointer = runtime.storage.at(entry.storage_stub_id).get();
        code = engine.RegisterGlobalProperty(entry.declaration.c_str(), pointer);
        if (code >= 0 && engine.GetGlobalPropertyCount() == before + 1U) {
            maps.globals.emplace(entry.logical_id, engine.registeredGlobalProps[before]);
            actual.index = before;
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "global property count did not advance exactly once");
        }
        break;
    }
    case registration_kind::global_function:
        code = engine.RegisterGlobalFunction(
            entry.declaration.c_str(), callable_stub(entry.convention),
            to_call_convention(entry.convention), nullptr, auxiliary());
        if (code >= 0) {
            auto* function = static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered global function is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, function);
            actual.engine_id = static_cast<std::uint32_t>(code);
        }
        break;
    case registration_kind::enum_type:
        code = engine.RegisterEnum(entry.declaration.c_str());
        if (code >= 0) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetTypeInfoById(code));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered enum is not reflectable");
            maps.types.emplace(entry.logical_id, type);
            maps.type_declarations.emplace(entry.logical_id, entry.declaration);
            actual.engine_id = static_cast<std::uint32_t>(code);
        }
        break;
    case registration_kind::enum_value: {
        asCTypeInfo* type = maps.types.at(entry.owner_type_id);
        const asUINT before = type->GetEnumValueCount();
        code = engine.RegisterEnumValue(owner_decl(), entry.name.c_str(), entry.enum_value);
        if (code >= 0 && type->GetEnumValueCount() == before + 1U) {
            actual.owner_engine_type_id = static_cast<std::uint32_t>(type->GetTypeId());
            actual.index = before;
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "enum value count did not advance exactly once");
        }
        break;
    }
    case registration_kind::funcdef:
        code = engine.RegisterFuncdef(entry.declaration.c_str());
        if (code >= 0) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetTypeInfoById(code));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered funcdef is not reflectable");
            maps.types.emplace(entry.logical_id, type);
            maps.type_declarations.emplace(entry.logical_id, entry.declaration);
            actual.engine_id = static_cast<std::uint32_t>(code);
        }
        break;
    case registration_kind::typedef_type: {
        const asUINT before = engine.GetTypedefCount();
        code = engine.RegisterTypedef(entry.name.c_str(), entry.target_declaration.c_str());
        if (code >= 0 && engine.GetTypedefCount() == before + 1U) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetTypedefByIndex(before));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered typedef is not reflectable");
            maps.types.emplace(entry.logical_id, type);
            maps.type_declarations.emplace(entry.logical_id, entry.name);
            actual.engine_id = static_cast<std::uint32_t>(code);
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "typedef count did not advance exactly once");
        }
        break;
    }
    case registration_kind::string_factory:
        runtime.strings = std::make_unique<string_pool>();
        code = engine.RegisterStringFactory(entry.declaration.c_str(), runtime.strings.get());
        if (code >= 0) actual.installed = true;
        break;
    case registration_kind::default_array_type:
        code = engine.RegisterDefaultArrayType(entry.declaration.c_str());
        if (code >= 0) actual.installed = true;
        break;
    }
    if (code < 0) {
        return fail(registry_replay_phase::register_entry, entry.ordinal, "AngelScript rejected a captured registration", code);
    }
    (void)profile;
    return {};
}

registry_replay_result apply_final_state(
    const post_bind_state& state,
    const std::size_t ordinal,
    replay_maps& maps) {
    switch (state.kind) {
    case post_bind_state_kind::object_type: {
        asCObjectType* type = object_type(maps, state.logical_id);
        if (type == nullptr || state.byte_size > max_object_bytes ||
            !valid_alignment(state.alignment) ||
            state.interface_type_ids.size() != state.interface_vft_offsets.size()) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "object type final state is invalid");
        }
        const auto resolve_object = [&](const std::optional<std::uint32_t>& id) -> asCObjectType* {
            return id.has_value() ? object_type(maps, *id) : nullptr;
        };
        type->size = static_cast<int>(state.byte_size);
        type->alignment = static_cast<int>(state.alignment);
        type->flags = state.flags;
        type->derivedFrom = resolve_object(state.base_type_id);
        type->shadowType = resolve_object(state.shadow_type_id);
        if ((state.base_type_id.has_value() && type->derivedFrom == nullptr) ||
            (state.shadow_type_id.has_value() && type->shadowType == nullptr)) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "object type base/shadow reference is invalid");
        }
        type->interfaces.SetLength(0U);
        type->interfaceVFTOffsets.SetLength(0U);
        for (std::size_t index = 0U; index < state.interface_type_ids.size(); ++index) {
            asCObjectType* interface_type = object_type(maps, state.interface_type_ids[index]);
            if (interface_type == nullptr) {
                return fail(registry_replay_phase::apply_post_bind_state, ordinal, "object type interface reference is invalid");
            }
            type->interfaces.PushLast(interface_type);
            type->interfaceVFTOffsets.PushLast(state.interface_vft_offsets[index]);
        }
        type->hasImplicitConstructors = state.has_implicit_constructors;
        type->acceptValueSubType = state.accepts_value_subtype;
        type->acceptRefSubType = state.accepts_reference_subtype;
        type->isInvalidGeneratedType = state.is_invalid_generated_type;
        break;
    }
    case post_bind_state_kind::object_property: {
        const auto iterator = maps.properties.find(state.logical_id);
        if (iterator == maps.properties.end() || state.byte_offset > max_offset ||
            state.composite_offset > max_offset || state.exposed_type > 255U) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "object property final state is invalid");
        }
        asCObjectProperty& property = *iterator->second;
        property.byteOffset = static_cast<int>(state.byte_offset);
        property.accessMask = state.access_mask;
        property.compositeOffset = static_cast<int>(state.composite_offset);
        property.isCompositeIndirect = state.is_composite_indirect;
        property.isPrivate = state.is_private;
        property.isProtected = state.is_protected;
        property.isAppBindProperty = state.is_app_bind_property;
        property.exposedType = state.exposed_type;
        break;
    }
    case post_bind_state_kind::function: {
        const auto iterator = maps.functions.find(state.logical_id);
        if (iterator == maps.functions.end() || (state.trait_bits & ~function_trait_mask) != 0U ||
            state.exposed_type > 255U ||
            state.hidden_argument_index.has_value() != state.hidden_argument_default.has_value()) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "function final state is invalid");
        }
        asCScriptFunction& function = *iterator->second;
        if (function.sysFuncIntf == nullptr && state.first_param != first_param_metadata::none) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "function metadata requires a system function interface");
        }
        function.traits.traits = state.trait_bits;
        function.exposedType = state.exposed_type;
        function.hiddenArgumentIndex = state.hidden_argument_index.has_value()
            ? static_cast<int8>(*state.hidden_argument_index) : static_cast<int8>(-1);
        function.hiddenArgumentDefault = state.hidden_argument_default.has_value()
            ? state.hidden_argument_default->c_str() : "";
        function.determinesOutputTypeArgumentIndex =
            state.determines_output_type_argument_index.has_value()
            ? static_cast<int8>(*state.determines_output_type_argument_index)
            : static_cast<int8>(-1);
        function.compileOutType = to_compile_out(state.compile_out);
        if (function.sysFuncIntf != nullptr) {
            function.sysFuncIntf->passFirstParamMetaData = to_first_param(state.first_param);
        }
        break;
    }
    case post_bind_state_kind::global_property: {
        const auto iterator = maps.globals.find(state.logical_id);
        if (iterator == maps.globals.end() ||
            state.is_pure_constant != state.pure_constant_value.has_value()) {
            return fail(registry_replay_phase::apply_post_bind_state, ordinal, "global property final state is invalid");
        }
        iterator->second->isPureConstant = state.is_pure_constant;
        iterator->second->storage = state.pure_constant_value.value_or(0U);
        break;
    }
    }
    return {};
}

bool verify_final_state(const post_bind_state& state, replay_maps& maps) {
    switch (state.kind) {
    case post_bind_state_kind::object_type: {
        asCObjectType* type = object_type(maps, state.logical_id);
        if (type == nullptr || type->size != static_cast<int>(state.byte_size) ||
            type->alignment != static_cast<int>(state.alignment) || type->flags != state.flags ||
            type->hasImplicitConstructors != state.has_implicit_constructors ||
            type->acceptValueSubType != state.accepts_value_subtype ||
            type->acceptRefSubType != state.accepts_reference_subtype ||
            type->isInvalidGeneratedType != state.is_invalid_generated_type ||
            type->interfaces.GetLength() != state.interface_type_ids.size() ||
            type->interfaceVFTOffsets.GetLength() != state.interface_vft_offsets.size()) return false;
        if ((type->derivedFrom == nullptr) != !state.base_type_id.has_value() ||
            (type->shadowType == nullptr) != !state.shadow_type_id.has_value()) return false;
        if (state.base_type_id.has_value() && type->derivedFrom != object_type(maps, *state.base_type_id)) return false;
        if (state.shadow_type_id.has_value() && type->shadowType != object_type(maps, *state.shadow_type_id)) return false;
        for (std::size_t index = 0U; index < state.interface_type_ids.size(); ++index) {
            if (type->interfaces[static_cast<asUINT>(index)] != object_type(maps, state.interface_type_ids[index]) ||
                type->interfaceVFTOffsets[static_cast<asUINT>(index)] != state.interface_vft_offsets[index]) return false;
        }
        return true;
    }
    case post_bind_state_kind::object_property: {
        const auto iterator = maps.properties.find(state.logical_id);
        if (iterator == maps.properties.end()) return false;
        const asCObjectProperty& property = *iterator->second;
        return property.byteOffset == static_cast<int>(state.byte_offset) &&
            property.accessMask == state.access_mask &&
            property.compositeOffset == static_cast<int>(state.composite_offset) &&
            property.isCompositeIndirect == state.is_composite_indirect &&
            property.isPrivate == state.is_private && property.isProtected == state.is_protected &&
            property.isAppBindProperty == state.is_app_bind_property &&
            property.exposedType == state.exposed_type;
    }
    case post_bind_state_kind::function: {
        const auto iterator = maps.functions.find(state.logical_id);
        if (iterator == maps.functions.end()) return false;
        const asCScriptFunction& function = *iterator->second;
        const int expected_hidden = state.hidden_argument_index.has_value() ? *state.hidden_argument_index : -1;
        const int expected_output = state.determines_output_type_argument_index.has_value()
            ? *state.determines_output_type_argument_index : -1;
        const char* expected_default = state.hidden_argument_default.has_value()
            ? state.hidden_argument_default->c_str() : "";
        const auto metadata = function.sysFuncIntf == nullptr
            ? asEFirstParamMetaData::None : function.sysFuncIntf->passFirstParamMetaData;
        return function.traits.traits == state.trait_bits &&
            function.exposedType == state.exposed_type &&
            function.hiddenArgumentIndex == expected_hidden &&
            function.hiddenArgumentDefault == expected_default &&
            function.determinesOutputTypeArgumentIndex == expected_output &&
            function.compileOutType == to_compile_out(state.compile_out) &&
            metadata == to_first_param(state.first_param);
    }
    case post_bind_state_kind::global_property: {
        const auto iterator = maps.globals.find(state.logical_id);
        return iterator != maps.globals.end() &&
            iterator->second->isPureConstant == state.is_pure_constant &&
            iterator->second->storage == state.pure_constant_value.value_or(0U);
    }
    }
    return false;
}

} // namespace

registry_runtime::registry_runtime() : impl_(std::make_unique<impl>()) {}
registry_runtime::~registry_runtime() = default;
registry_runtime::registry_runtime(registry_runtime&&) noexcept = default;
registry_runtime& registry_runtime::operator=(registry_runtime&&) noexcept = default;

registry_replay_result replay_registry(
    asIScriptEngine& interface,
    const registry_profile& profile,
    registry_runtime& runtime) {
    registry_replay_result result = validate_profile(profile);
    if (!result.succeeded()) return result;

    auto& engine = static_cast<asCScriptEngine&>(interface);
    if (runtime.impl_->bound) {
        return fail(registry_replay_phase::validate_profile, no_ordinal, "registry replay requires a fresh runtime");
    }
    if (engine.GetObjectTypeCount() != 0U || engine.GetEnumCount() != 0U ||
        engine.GetFuncdefCount() != 0U || engine.GetTypedefCount() != 0U ||
        engine.GetGlobalFunctionCount() != 0U || engine.GetGlobalPropertyCount() != 0U ||
        engine.GetModuleCount() != 0U || engine.GetStringFactoryReturnTypeId(nullptr) != asNO_FUNCTION ||
        engine.GetDefaultArrayTypeId() != asINVALID_TYPE) {
        return fail(registry_replay_phase::validate_profile, no_ordinal, "registry replay requires a fresh engine");
    }

    auto prepared = std::make_unique<registry_runtime::impl>();
    for (const host_stub& stub : profile.host_stubs) {
        if (stub.kind == host_stub_kind::storage) {
            const std::size_t size = std::max<std::size_t>(stub.byte_len, 1U);
            void* pointer = _aligned_malloc(size, stub.alignment);
            if (pointer == nullptr) return fail(registry_replay_phase::validate_profile, stub.stub_id, "storage stub allocation failed", asOUT_OF_MEMORY);
            std::memset(pointer, 0, size);
            prepared->storage.emplace(stub.stub_id, aligned_pointer(pointer));
        } else if (stub.kind == host_stub_kind::object) {
            prepared->objects.emplace(stub.stub_id, std::make_unique<std::byte>());
        }
    }

    for (const engine_property_setting& setting : profile.engine_properties) {
        const int code = engine.SetEngineProperty(to_engine_property(setting.property), setting.value);
        if (code < 0 || engine.GetEngineProperty(to_engine_property(setting.property)) != setting.value) {
            return fail(registry_replay_phase::apply_engine_properties, setting.ordinal, "engine property did not apply exactly", code);
        }
    }

    replay_maps maps;
    for (std::size_t index = 0U; index < profile.registrations.size(); ++index) {
        const registration_entry& entry = profile.registrations[index];
        result = apply_context(engine, entry);
        if (!result.succeeded()) return result;
        registration_result actual;
        result = register_one(engine, profile, entry, *prepared, maps, actual);
        if (!result.succeeded()) return result;
        if (!same_result(actual, profile.expected_results[index])) {
            return fail(registry_replay_phase::verify_registration_result, index, "registration result differs from captured post-bind identity");
        }
    }

    for (std::size_t index = 0U; index < profile.final_states.size(); ++index) {
        result = apply_final_state(profile.final_states[index], index, maps);
        if (!result.succeeded()) return result;
    }
    for (std::size_t index = 0U; index < profile.final_states.size(); ++index) {
        if (!verify_final_state(profile.final_states[index], maps)) {
            return fail(registry_replay_phase::verify_post_bind_state, index, "post-bind state did not round-trip exactly");
        }
    }

    prepared->bound = true;
    runtime.impl_ = std::move(prepared);
    return {};
}

} // namespace gore::as::standalone
