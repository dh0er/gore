#include "gore_as_standalone/registry_profile.hpp"

#include "as_callfunc.h"
#include "as_objecttype.h"
#include "as_property.h"
#include "as_scriptengine.h"
#include "as_scriptfunction.h"
#include "as_string.h"
#include "as_typeinfo.h"

#include <algorithm>
#include <array>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <malloc.h>
#include <map>
#include <mutex>
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
        return (character < 0x20U && character != '\t' && character != '\n' &&
                character != '\r') ||
            character == 0x7fU;
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

bool valid_fixed_operations(const fixed_type_operations& operations) noexcept {
    return operations.value_size <= max_object_bytes &&
        valid_alignment(operations.value_alignment);
}

bool neutral_fixed_operations(const fixed_type_operations& operations) noexcept {
    return !operations.can_create_property && !operations.never_requires_gc &&
        !operations.requires_property && !operations.can_be_template_subtype &&
        !operations.can_construct &&
        operations.need_construct && !operations.can_destruct && operations.need_destruct &&
        !operations.can_copy && operations.need_copy && !operations.can_compare &&
        !operations.can_hash_value && operations.value_size == 0U &&
        operations.value_alignment == 1U && !operations.is_object_pointer;
}

bool valid_type_operations(const type_operations& operations, const bool allow_container) noexcept {
    switch (operations.kind) {
    case type_operations_kind::unavailable:
        return neutral_fixed_operations(operations.fixed);
    case type_operations_kind::fixed:
        return valid_fixed_operations(operations.fixed);
    case type_operations_kind::t_array:
    case type_operations_kind::t_map:
    case type_operations_kind::t_set:
    case type_operations_kind::t_optional:
        return allow_container && neutral_fixed_operations(operations.fixed);
    }
    return false;
}

bool is_container_operations(const type_operations_kind kind) noexcept {
    return kind == type_operations_kind::t_array || kind == type_operations_kind::t_map ||
        kind == type_operations_kind::t_set || kind == type_operations_kind::t_optional;
}

bool is_container_adapter(const template_validation_adapter adapter) noexcept {
    return adapter == template_validation_adapter::t_array ||
        adapter == template_validation_adapter::t_map ||
        adapter == template_validation_adapter::t_set ||
        adapter == template_validation_adapter::t_optional;
}

bool operations_match_adapter(
    const type_operations_kind operations,
    const template_validation_adapter adapter) noexcept {
    return (operations == type_operations_kind::t_array &&
            adapter == template_validation_adapter::t_array) ||
        (operations == type_operations_kind::t_map &&
         adapter == template_validation_adapter::t_map) ||
        (operations == type_operations_kind::t_set &&
         adapter == template_validation_adapter::t_set) ||
        (operations == type_operations_kind::t_optional &&
         adapter == template_validation_adapter::t_optional);
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

// ABI projections of the two UE value containers used by the sealed qualification corpus.
// They intentionally implement only the donor operations required by those exact probes.
struct qualification_script_array {
    void* data = nullptr;
    std::int32_t count = 0;
    std::int32_t capacity = 0;
};
static_assert(sizeof(qualification_script_array) == 16U);
static_assert(alignof(qualification_script_array) == 8U);

struct qualification_fstring {
    char16_t* data = nullptr;
    std::int32_t count = 0; // UE FString includes its terminating NUL when non-empty.
    std::int32_t capacity = 0;

    qualification_fstring& assign(const qualification_fstring& other);
};
static_assert(sizeof(qualification_fstring) == 16U);
static_assert(alignof(qualification_fstring) == 8U);

struct qualification_fname {
    std::uint32_t comparison_index = 0U;
    std::uint32_t number = 0U;
};
static_assert(sizeof(qualification_fname) == 8U);
static_assert(alignof(qualification_fname) == 4U);

constexpr std::int32_t qualification_max_array_values = 1024;
constexpr std::int32_t qualification_max_utf16_units = 1024 * 1024;

bool qualification_array_metadata_allowed(const asCObjectType* const type) noexcept {
    if (type == nullptr || type->GetName() == nullptr ||
        std::string_view(type->GetName()) != "TArray" || type->GetSize() != 16U ||
        type->GetSubTypeCount() != 1U) return false;
    const int subtype_id = type->GetSubTypeId(0U);
    if (subtype_id == asTYPEID_INT32) return true;
    const asITypeInfo* const subtype = type->GetSubType(0U);
    return subtype != nullptr && (subtype->GetFlags() & asOBJ_TEMPLATE_SUBTYPE) != 0U;
}

void qualification_array_construct(qualification_script_array* array) {
    if (array != nullptr) new(array) qualification_script_array();
}

void qualification_array_destruct(qualification_script_array& array, asCObjectType*) {
    _aligned_free(array.data);
    array = {};
}

qualification_script_array& qualification_array_assign(
    qualification_script_array& destination, asCObjectType* type,
    qualification_script_array& source) {
    if (!qualification_array_metadata_allowed(type) || source.count < 0 ||
        source.count > qualification_max_array_values || source.capacity < source.count ||
        source.capacity > qualification_max_array_values ||
        (source.count != 0 && source.data == nullptr) || destination.count < 0 ||
        destination.capacity < destination.count ||
        destination.capacity > qualification_max_array_values ||
        (destination.capacity != 0 && destination.data == nullptr)) {
        if (asIScriptContext* context = asGetActiveContext()) {
            context->SetException("qualification TArray<int32> assignment contract violation");
        }
        return destination;
    }
    if (source.count > destination.capacity) {
        void* const replacement = _aligned_realloc(
            destination.data, static_cast<std::size_t>(source.count) * sizeof(std::int32_t),
            alignof(void*));
        if (replacement == nullptr && source.count != 0) {
            if (asIScriptContext* context = asGetActiveContext()) {
                context->SetException("qualification TArray<int32> assignment allocation failed");
            }
            return destination;
        }
        destination.data = replacement;
        destination.capacity = source.count;
    }
    if (source.count != 0) {
        std::memcpy(destination.data, source.data,
            static_cast<std::size_t>(source.count) * sizeof(std::int32_t));
    }
    destination.count = source.count;
    return destination;
}

void qualification_array_set_num(
    qualification_script_array& array, asCObjectType* type, const std::int32_t count) {
    if (!qualification_array_metadata_allowed(type) || count < 0 ||
        count > qualification_max_array_values || array.count < 0 ||
        array.capacity < array.count || array.capacity > qualification_max_array_values ||
        (array.capacity != 0 && array.data == nullptr)) {
        if (asIScriptContext* context = asGetActiveContext()) {
            const std::string diagnostic = "qualification TArray<int32> SetNum contract violation (subtype=" +
                std::to_string(type == nullptr ? -1 : type->GetSubTypeId(0U)) +
                ", count=" + std::to_string(count) + ", current=" +
                std::to_string(array.count) + ", capacity=" +
                std::to_string(array.capacity) + ")";
            context->SetException(diagnostic.c_str());
        }
        return;
    }
    if (count > array.capacity) {
        void* const replacement = _aligned_realloc(array.data,
            static_cast<std::size_t>(count) * sizeof(std::int32_t), alignof(void*));
        if (replacement == nullptr && count != 0) {
            if (asIScriptContext* context = asGetActiveContext()) {
                context->SetException("qualification TArray<int32> SetNum allocation failed");
            }
            return;
        }
        array.data = replacement;
        array.capacity = count;
    }
    if (count > array.count) {
        std::memset(static_cast<std::int32_t*>(array.data) + array.count, 0,
            static_cast<std::size_t>(count - array.count) * sizeof(std::int32_t));
    }
    array.count = count;
}

void qualification_fstring_construct(qualification_fstring* value) {
    if (value != nullptr) new(value) qualification_fstring();
}

void qualification_fstring_copy_construct(
    qualification_fstring* value, const qualification_fstring& other) {
    if (value == nullptr) return;
    new(value) qualification_fstring();
    value->assign(other);
}

void qualification_fstring_destruct(qualification_fstring& value) {
    _aligned_free(value.data);
    value = {};
}

qualification_fstring& qualification_fstring::assign(const qualification_fstring& other) {
    if (&other == this) return *this;
    if (capacity < 0 || (capacity != 0 && data == nullptr) || other.count < 0 ||
        other.capacity < other.count ||
        other.count > qualification_max_utf16_units ||
        (other.count != 0 && (other.data == nullptr || other.data[other.count - 1] != u'\0'))) {
        if (asIScriptContext* context = asGetActiveContext()) {
            context->SetException("qualification FString assignment contract violation");
        }
        return *this;
    }
    if (other.count > capacity) {
        void* const replacement = _aligned_realloc(
            data, static_cast<std::size_t>(other.count) * sizeof(char16_t), alignof(char16_t));
        if (replacement == nullptr && other.count != 0) {
            if (asIScriptContext* context = asGetActiveContext()) {
                context->SetException("qualification FString assignment allocation failed");
            }
            return *this;
        }
        data = static_cast<char16_t*>(replacement);
        capacity = other.count;
    }
    if (other.count != 0) {
        std::memcpy(data, other.data, static_cast<std::size_t>(other.count) * sizeof(char16_t));
    }
    count = other.count;
    return *this;
}

bool qualification_fname_equals(
    const qualification_fname& left, const qualification_fname& right) noexcept {
    return left.comparison_index == right.comparison_index && left.number == right.number;
}

void qualification_fname_construct(qualification_fname* value) {
    if (value != nullptr) new(value) qualification_fname();
}

void qualification_fname_copy_construct(
    qualification_fname* value, const qualification_fname& other) {
    if (value != nullptr) new(value) qualification_fname(other);
}

const qualification_fname& qualification_static_name(std::int32_t id);

void qualification_array_construct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_array_construct(static_cast<qualification_script_array*>(arguments[0]));
}
void qualification_array_destruct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_array_destruct(*static_cast<qualification_script_array*>(arguments[0]),
        static_cast<asCObjectType*>(arguments[1]));
}
void qualification_array_assign_caller(
    asFUNCTION_t, void** arguments, void* return_value) {
    auto& result = qualification_array_assign(
        *static_cast<qualification_script_array*>(arguments[0]),
        static_cast<asCObjectType*>(arguments[1]),
        *static_cast<qualification_script_array*>(arguments[2]));
    *static_cast<void**>(return_value) = &result;
}
void qualification_array_set_num_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_array_set_num(*static_cast<qualification_script_array*>(arguments[0]),
        static_cast<asCObjectType*>(arguments[1]),
        *static_cast<const std::int32_t*>(arguments[2]));
}
void qualification_fname_construct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_fname_construct(static_cast<qualification_fname*>(arguments[0]));
}
void qualification_fname_copy_construct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_fname_copy_construct(static_cast<qualification_fname*>(arguments[0]),
        *static_cast<const qualification_fname*>(arguments[1]));
}
void qualification_fname_equals_caller(
    asFUNCTION_t, void** arguments, void* return_value) {
    *static_cast<asDWORD*>(return_value) = qualification_fname_equals(
        *static_cast<const qualification_fname*>(arguments[0]),
        *static_cast<const qualification_fname*>(arguments[1])) ? 1U : 0U;
}
void qualification_static_name_caller(
    asFUNCTION_t, void** arguments, void* return_value) {
    *static_cast<const qualification_fname**>(return_value) =
        &qualification_static_name(*static_cast<const std::int32_t*>(arguments[0]));
}
void qualification_fstring_construct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_fstring_construct(static_cast<qualification_fstring*>(arguments[0]));
}
void qualification_fstring_copy_construct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_fstring_copy_construct(static_cast<qualification_fstring*>(arguments[0]),
        *static_cast<const qualification_fstring*>(arguments[1]));
}
void qualification_fstring_destruct_caller(asFUNCTION_t, void** arguments, void*) {
    qualification_fstring_destruct(*static_cast<qualification_fstring*>(arguments[0]));
}
void qualification_fstring_assign_caller(
    asMETHOD_t, void** arguments, void* return_value) {
    auto& result = static_cast<qualification_fstring*>(arguments[0])->assign(
        *static_cast<const qualification_fstring*>(arguments[1]));
    *static_cast<void**>(return_value) = &result;
}

bool validate_class_template(asITypeInfo* type, asCString* error);

bool validate_array_template(asITypeInfo* type, asCString* error);
bool validate_map_template(asITypeInfo* type, asCString* error);
bool validate_set_template(asITypeInfo* type, asCString* error);
bool validate_optional_template(asITypeInfo* type, asCString* error);

asSFuncPtr template_callback(const template_validation_adapter adapter) {
    switch (adapter) {
    case template_validation_adapter::t_subclass_of:
    case template_validation_adapter::t_object_ptr:
    case template_validation_adapter::t_weak_object_ptr:
    case template_validation_adapter::t_soft_object_ptr:
    case template_validation_adapter::t_soft_class_ptr:
        return asFUNCTION(validate_class_template);
    case template_validation_adapter::t_array:
        return asFUNCTION(validate_array_template);
    case template_validation_adapter::t_map:
        return asFUNCTION(validate_map_template);
    case template_validation_adapter::t_set:
        return asFUNCTION(validate_set_template);
    case template_validation_adapter::t_optional:
        return asFUNCTION(validate_optional_template);
    case template_validation_adapter::none:
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

bool qualification_utf8_to_utf16(
    const std::string_view input, std::vector<char16_t>& output) {
    output.clear();
    output.reserve(input.size() + 1U);
    for (std::size_t offset = 0U; offset < input.size();) {
        const auto first = static_cast<unsigned char>(input[offset]);
        std::uint32_t codepoint = 0U;
        std::size_t width = 0U;
        if (first < 0x80U) { codepoint = first; width = 1U; }
        else if ((first & 0xe0U) == 0xc0U) { codepoint = first & 0x1fU; width = 2U; }
        else if ((first & 0xf0U) == 0xe0U) { codepoint = first & 0x0fU; width = 3U; }
        else if ((first & 0xf8U) == 0xf0U) { codepoint = first & 0x07U; width = 4U; }
        else return false;
        if (offset + width > input.size()) return false;
        for (std::size_t index = 1U; index < width; ++index) {
            const auto continuation = static_cast<unsigned char>(input[offset + index]);
            if ((continuation & 0xc0U) != 0x80U) return false;
            codepoint = (codepoint << 6U) | (continuation & 0x3fU);
        }
        if ((width == 2U && codepoint < 0x80U) ||
            (width == 3U && codepoint < 0x800U) ||
            (width == 4U && codepoint < 0x10000U) ||
            codepoint > 0x10ffffU || (codepoint >= 0xd800U && codepoint <= 0xdfffU)) {
            return false;
        }
        if (codepoint < 0x10000U) {
            output.push_back(static_cast<char16_t>(codepoint));
        } else {
            codepoint -= 0x10000U;
            output.push_back(static_cast<char16_t>(0xd800U + (codepoint >> 10U)));
            output.push_back(static_cast<char16_t>(0xdc00U + (codepoint & 0x3ffU)));
        }
        if (output.size() >= static_cast<std::size_t>(qualification_max_utf16_units)) return false;
        offset += width;
    }
    output.push_back(u'\0');
    return true;
}

bool qualification_utf16_to_utf8(
    const qualification_fstring& input, std::string& output) {
    output.clear();
    if (input.count == 0) return input.data == nullptr && input.capacity >= 0;
    if (input.count < 1 || input.count > qualification_max_utf16_units ||
        input.capacity < input.count || input.data == nullptr ||
        input.data[input.count - 1] != u'\0') return false;
    for (std::int32_t index = 0; index < input.count - 1; ++index) {
        std::uint32_t codepoint = input.data[index];
        if (codepoint >= 0xd800U && codepoint <= 0xdbffU) {
            if (++index >= input.count - 1) return false;
            const std::uint32_t low = input.data[index];
            if (low < 0xdc00U || low > 0xdfffU) return false;
            codepoint = 0x10000U + ((codepoint - 0xd800U) << 10U) + (low - 0xdc00U);
        } else if (codepoint >= 0xdc00U && codepoint <= 0xdfffU) {
            return false;
        }
        if (codepoint < 0x80U) output.push_back(static_cast<char>(codepoint));
        else if (codepoint < 0x800U) {
            output.push_back(static_cast<char>(0xc0U | (codepoint >> 6U)));
            output.push_back(static_cast<char>(0x80U | (codepoint & 0x3fU)));
        } else if (codepoint < 0x10000U) {
            output.push_back(static_cast<char>(0xe0U | (codepoint >> 12U)));
            output.push_back(static_cast<char>(0x80U | ((codepoint >> 6U) & 0x3fU)));
            output.push_back(static_cast<char>(0x80U | (codepoint & 0x3fU)));
        } else {
            output.push_back(static_cast<char>(0xf0U | (codepoint >> 18U)));
            output.push_back(static_cast<char>(0x80U | ((codepoint >> 12U) & 0x3fU)));
            output.push_back(static_cast<char>(0x80U | ((codepoint >> 6U) & 0x3fU)));
            output.push_back(static_cast<char>(0x80U | (codepoint & 0x3fU)));
        }
    }
    return true;
}

class string_pool final : public asIStringFactory {
public:
    explicit string_pool(const bool qualification_fstring) noexcept
        : qualification_fstring_(qualification_fstring) {}

    const void* GetStringConstant(const char* data, const asUINT length) override {
        const std::string key(data, static_cast<std::size_t>(length));
        auto value = std::make_unique<record>();
        value->bytes = key;
        if (qualification_fstring_) {
            std::vector<char16_t> utf16;
            if (!qualification_utf8_to_utf16(key, utf16)) {
                return nullptr;
            }
            qualification_fstring source;
            source.data = utf16.data();
            source.count = static_cast<std::int32_t>(utf16.size());
            source.capacity = source.count;
            value->value.assign(source);
        }
        value->references = 1U;
        const void* const result = qualification_fstring_
            ? static_cast<const void*>(&value->value)
            : static_cast<const void*>(value.get());
        if (!values_.emplace(result, std::move(value)).second) {
            return nullptr;
        }
        return result;
    }

    int ReleaseStringConstant(const void* value) override {
        if (value == nullptr) {
            return asINVALID_ARG;
        }
        const auto iterator = values_.find(value);
        if (iterator == values_.end() || iterator->second->references == 0U) {
            return asINVALID_ARG;
        }
        --iterator->second->references;
        if (iterator->second->references == 0U) {
            values_.erase(iterator);
        }
        return asSUCCESS;
    }

    int GetRawStringData(const void* value, char* data, asUINT* length) const override {
        if (value == nullptr || length == nullptr) {
            return asINVALID_ARG;
        }
        const auto iterator = values_.find(value);
        if (iterator == values_.end() ||
            iterator->second->bytes.size() > std::numeric_limits<asUINT>::max()) {
            return asINVALID_ARG;
        }
        const auto actual_length = static_cast<asUINT>(iterator->second->bytes.size());
        if (data != nullptr) {
            std::memcpy(data, iterator->second->bytes.data(), actual_length);
        }
        *length = actual_length;
        return asSUCCESS;
    }

    [[nodiscard]] bool contains_qualification_value(const void* const value) const noexcept {
        return qualification_fstring_ && value != nullptr && values_.find(value) != values_.end();
    }

private:
    struct record {
        qualification_fstring value;
        std::string bytes;
        std::size_t references = 0U;
        ~record() { _aligned_free(value.data); }
    };
    bool qualification_fstring_ = false;
    // The target FString factory returns a distinct stable constant object for each request,
    // including repeated text. Cache serialization keys string globals by those object addresses;
    // interning by spelling therefore collapses real rows and changes the complete graph export.
    std::unordered_map<const void*, std::unique_ptr<record>> values_;
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
        profile.primitive_operations.size() != 11U ||
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
    constexpr std::array<primitive_type, 11U> primitive_order = {
        primitive_type::bool_type, primitive_type::int8, primitive_type::int16,
        primitive_type::int32, primitive_type::int64, primitive_type::uint8,
        primitive_type::uint16, primitive_type::uint32, primitive_type::uint64,
        primitive_type::float32, primitive_type::float64};
    for (std::size_t index = 0U; index < profile.primitive_operations.size(); ++index) {
        const primitive_type_operations& primitive = profile.primitive_operations[index];
        if (primitive.ordinal != index || primitive.primitive != primitive_order[index] ||
            !valid_fixed_operations(primitive.operations) ||
            primitive.operations.value_size == 0U) {
            return fail(
                registry_replay_phase::validate_profile, index,
                "primitive type-operation order or descriptor is invalid");
        }
    }
    if (!valid_fixed_operations(profile.dynamic_script_operations.delegate) ||
        !valid_fixed_operations(profile.dynamic_script_operations.multicast_delegate) ||
        profile.dynamic_script_operations.delegate.value_size == 0U ||
        profile.dynamic_script_operations.multicast_delegate.value_size == 0U) {
        return fail(
            registry_replay_phase::validate_profile, no_ordinal,
            "dynamic script type-operation descriptor is invalid");
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
    std::unordered_map<std::uint32_t, type_operations_kind> operations_by_type;
    std::unordered_set<std::uint32_t> container_callbacks;
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
        const bool produces_operations = entry.kind == registration_kind::object_type ||
            entry.kind == registration_kind::interface_type ||
            entry.kind == registration_kind::enum_type ||
            entry.kind == registration_kind::funcdef;
        if (!produces_operations &&
            (entry.operations.kind != type_operations_kind::unavailable ||
             !valid_type_operations(entry.operations, false))) {
            return fail(
                registry_replay_phase::validate_profile, index,
                "non-type registration carries a type-operation descriptor");
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
                !valid_type_operations(entry.operations, true) ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "object type descriptor is invalid");
            }
            object_types.insert(entry.logical_id);
            operations_by_type.emplace(entry.logical_id, entry.operations.kind);
            expected_final_types.insert(entry.logical_id);
            break;
        case registration_kind::interface_type:
            if (!valid_text(entry.declaration) ||
                !valid_type_operations(entry.operations, false) ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "interface descriptor is invalid");
            }
            interface_types.insert(entry.logical_id);
            operations_by_type.emplace(entry.logical_id, entry.operations.kind);
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
                    "template validator implementation is unavailable",
                    asNOT_SUPPORTED);
            }
            if (is_container_adapter(entry.validation_adapter)) {
                const auto operations = operations_by_type.find(entry.owner_type_id);
                if (operations == operations_by_type.end() ||
                    !operations_match_adapter(operations->second, entry.validation_adapter) ||
                    !container_callbacks.insert(entry.owner_type_id).second) {
                    return fail(
                        registry_replay_phase::validate_profile, index,
                        "container template validator does not match its owner type operations");
                }
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
            if (!valid_text(entry.declaration) ||
                !valid_type_operations(entry.operations, false) ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "enum descriptor is invalid");
            }
            enum_types.insert(entry.logical_id);
            operations_by_type.emplace(entry.logical_id, entry.operations.kind);
            break;
        case registration_kind::enum_value:
            if (enum_types.count(entry.owner_type_id) == 0U || !valid_identifier(entry.name)) {
                return fail(registry_replay_phase::validate_profile, index, "enum value descriptor is invalid");
            }
            break;
        case registration_kind::funcdef:
            if (!valid_text(entry.declaration) ||
                entry.operations.kind != type_operations_kind::unavailable ||
                !valid_type_operations(entry.operations, false) ||
                !type_ids.insert(entry.logical_id).second) {
                return fail(registry_replay_phase::validate_profile, index, "funcdef descriptor is invalid");
            }
            operations_by_type.emplace(entry.logical_id, entry.operations.kind);
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
    for (const auto& [type_id, operations] : operations_by_type) {
        if (is_container_operations(operations) &&
            container_callbacks.count(type_id) == 0U) {
            return fail(
                registry_replay_phase::validate_profile, no_ordinal,
                "container type operations have no matching template callback");
        }
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

struct template_operation_record {
    type_operations_kind kind = type_operations_kind::unavailable;
    fixed_type_operations first;
    fixed_type_operations second;
    bool valid = false;
    asIScriptFunction* compare_function = nullptr;
    asIScriptFunction* hash_function = nullptr;
};

template_operation_record* existing_template_record(
    class registry_runtime::impl& runtime,
    asITypeInfo& type);

enum class qualification_adapter_role {
    array_construct,
    array_destruct,
    array_assign,
    array_set_num,
    fname_construct,
    fname_copy_construct,
    fname_equals,
    fname_static_name,
    fstring_construct,
    fstring_copy_construct,
    fstring_destruct,
    fstring_assign,
    fstring_factory,
};

} // namespace

class registry_runtime::impl final {
public:
    ~impl();

    bool bound = false;
    asIScriptEngine* engine = nullptr;
    std::unordered_map<std::uint32_t, aligned_pointer> storage;
    std::unordered_map<std::uint32_t, std::unique_ptr<std::byte>> objects;
    std::unique_ptr<string_pool> strings;
    std::array<fixed_type_operations, 11U> primitive_operations;
    dynamic_script_type_operations dynamic_script_operations;
    std::unordered_map<const asITypeInfo*, type_operations> type_operations_by_pointer;
    std::unordered_map<std::string, type_operations> type_operations_by_name;
    std::unordered_map<const asITypeInfo*, dynamic_script_type_category> dynamic_script_categories;
    std::vector<std::unique_ptr<template_operation_record>> template_records;
    std::unordered_set<const template_operation_record*> template_record_pointers;
    std::mutex operations_mutex;
    qualification_runtime_kind qualification_kind = qualification_runtime_kind::none;
    std::vector<std::string> qualification_static_name_identities;
    std::vector<qualification_fname> qualification_static_names;
    std::unordered_set<const asIScriptFunction*> qualification_functions;
    std::unordered_map<qualification_adapter_role, const asIScriptFunction*>
        qualification_roles;
    std::unordered_set<std::uint32_t> qualification_type_ids;
    std::unordered_set<const asITypeInfo*> qualification_types;
    bool qualification_string_factory = false;
};

namespace {

std::mutex runtime_registry_mutex;
std::unordered_map<asIScriptEngine*, registry_runtime::impl*> runtime_registry;

void unregister_runtime(registry_runtime::impl& runtime) noexcept {
    if (runtime.engine == nullptr) return;
    const std::lock_guard<std::mutex> lock(runtime_registry_mutex);
    const auto iterator = runtime_registry.find(runtime.engine);
    if (iterator != runtime_registry.end() && iterator->second == &runtime) {
        runtime_registry.erase(iterator);
    }
    runtime.engine = nullptr;
}

bool register_runtime(asIScriptEngine& engine, registry_runtime::impl& runtime) {
    const std::lock_guard<std::mutex> lock(runtime_registry_mutex);
    if (!runtime_registry.emplace(&engine, &runtime).second) return false;
    runtime.engine = &engine;
    return true;
}

registry_runtime::impl* find_runtime(asIScriptEngine* engine) {
    if (engine == nullptr) return nullptr;
    const std::lock_guard<std::mutex> lock(runtime_registry_mutex);
    const auto iterator = runtime_registry.find(engine);
    return iterator == runtime_registry.end() ? nullptr : iterator->second;
}

const qualification_fname& qualification_static_name(const std::int32_t id) {
    static const qualification_fname invalid{};
    asIScriptContext* const context = asGetActiveContext();
    registry_runtime::impl* const runtime =
        context == nullptr ? nullptr : find_runtime(context->GetEngine());
    if (runtime == nullptr ||
        runtime->qualification_kind != qualification_runtime_kind::fname_equivalence ||
        id < 0 || static_cast<std::size_t>(id) >= runtime->qualification_static_names.size() ||
        runtime->qualification_static_name_identities[static_cast<std::size_t>(id)].empty()) {
        if (context != nullptr) {
            context->SetException("qualification FName static-name identity is unavailable");
        }
        return invalid;
    }
    return runtime->qualification_static_names[static_cast<std::size_t>(id)];
}

bool qualification_object_type_matches(
    const registry_runtime::impl& runtime,
    const registration_entry& entry) noexcept {
    if ((entry.flags & asOBJ_VALUE) == 0U) return false;
    switch (runtime.qualification_kind) {
    case qualification_runtime_kind::t_array_int32:
        return entry.declaration == "TArray<class T>" && entry.byte_size == 16U &&
            entry.alignment == 8U && entry.operations.kind == type_operations_kind::t_array;
    case qualification_runtime_kind::fname_equivalence:
        return entry.declaration == "FName" && entry.byte_size == 8U && entry.alignment == 8U &&
            entry.operations.kind == type_operations_kind::fixed &&
            entry.operations.fixed.value_size == 8U &&
            entry.operations.fixed.value_alignment == 4U &&
            entry.operations.fixed.can_construct && entry.operations.fixed.can_copy &&
            entry.operations.fixed.can_compare;
    case qualification_runtime_kind::fstring_roundtrip:
        return entry.declaration == "FString" && entry.byte_size == 16U && entry.alignment == 8U &&
            entry.operations.kind == type_operations_kind::fixed &&
            entry.operations.fixed.value_size == 16U &&
            entry.operations.fixed.value_alignment == 8U &&
            entry.operations.fixed.can_construct && entry.operations.fixed.need_construct &&
            entry.operations.fixed.can_destruct && entry.operations.fixed.need_destruct &&
            entry.operations.fixed.can_copy && entry.operations.fixed.need_copy;
    case qualification_runtime_kind::none:
        return false;
    }
    return false;
}

struct qualification_adapter {
    asSFuncPtr function;
    asFunctionCaller caller;
    qualification_adapter_role role;
};

std::optional<qualification_adapter> qualification_adapter_for(
    registry_runtime::impl& runtime,
    const registration_entry& entry,
    const std::string_view owner_declaration) {
    const bool qualified_owner =
        runtime.qualification_type_ids.count(entry.owner_type_id) != 0U;
    switch (runtime.qualification_kind) {
    case qualification_runtime_kind::t_array_int32:
        if (!qualified_owner || owner_declaration != "TArray<T>" ||
            entry.convention != call_convention::cdecl_object_first) return std::nullopt;
        if (entry.kind == registration_kind::object_behaviour &&
            entry.behaviour == object_behaviour::construct && entry.declaration == "void f()") {
            return qualification_adapter{asFUNCTION(qualification_array_construct),
                asFunctionCaller(&qualification_array_construct_caller),
                qualification_adapter_role::array_construct};
        }
        if (entry.kind == registration_kind::object_behaviour &&
            entry.behaviour == object_behaviour::destruct && entry.declaration == "void f()") {
            return qualification_adapter{asFUNCTION(qualification_array_destruct),
                asFunctionCaller(&qualification_array_destruct_caller),
                qualification_adapter_role::array_destruct};
        }
        if (entry.kind == registration_kind::object_method &&
            entry.declaration == "TArray<T>& opAssign(const TArray<T>& Other)") {
            return qualification_adapter{asFUNCTION(qualification_array_assign),
                asFunctionCaller(&qualification_array_assign_caller),
                qualification_adapter_role::array_assign};
        }
        if (entry.kind == registration_kind::object_method &&
            entry.declaration == "void SetNum(int32 __any_implicit_integer NewNum = 0)") {
            return qualification_adapter{asFUNCTION(qualification_array_set_num),
                asFunctionCaller(&qualification_array_set_num_caller),
                qualification_adapter_role::array_set_num};
        }
        break;
    case qualification_runtime_kind::fname_equivalence:
        if (entry.kind == registration_kind::global_function &&
            entry.context.name_space.empty() && entry.convention == call_convention::cdecl_call &&
            entry.declaration == "const FName& __STATIC_NAME(int Id) no_discard") {
            return qualification_adapter{asFUNCTION(qualification_static_name),
                asFunctionCaller(&qualification_static_name_caller),
                qualification_adapter_role::fname_static_name};
        }
        if (!qualified_owner || owner_declaration != "FName" ||
            entry.convention != call_convention::cdecl_object_first) return std::nullopt;
        if (entry.kind == registration_kind::object_behaviour &&
            entry.behaviour == object_behaviour::construct && entry.declaration == "void f()") {
            return qualification_adapter{asFUNCTION(qualification_fname_construct),
                asFunctionCaller(&qualification_fname_construct_caller),
                qualification_adapter_role::fname_construct};
        }
        if (entry.kind == registration_kind::object_behaviour &&
            entry.behaviour == object_behaviour::construct &&
            entry.declaration == "void f(const FName& Other)") {
            return qualification_adapter{asFUNCTION(qualification_fname_copy_construct),
                asFunctionCaller(&qualification_fname_copy_construct_caller),
                qualification_adapter_role::fname_copy_construct};
        }
        if (entry.kind == registration_kind::object_method &&
            entry.declaration == "bool opEquals(const FName& Other) const") {
            return qualification_adapter{asFUNCTION(qualification_fname_equals),
                asFunctionCaller(&qualification_fname_equals_caller),
                qualification_adapter_role::fname_equals};
        }
        break;
    case qualification_runtime_kind::fstring_roundtrip:
        if (!qualified_owner || owner_declaration != "FString") return std::nullopt;
        if (entry.kind == registration_kind::object_behaviour &&
            entry.convention == call_convention::cdecl_object_first &&
            entry.behaviour == object_behaviour::construct && entry.declaration == "void f()") {
            return qualification_adapter{asFUNCTION(qualification_fstring_construct),
                asFunctionCaller(&qualification_fstring_construct_caller),
                qualification_adapter_role::fstring_construct};
        }
        if (entry.kind == registration_kind::object_behaviour &&
            entry.convention == call_convention::cdecl_object_first &&
            entry.behaviour == object_behaviour::construct &&
            entry.declaration == "void f(const FString& Other)") {
            return qualification_adapter{asFUNCTION(qualification_fstring_copy_construct),
                asFunctionCaller(&qualification_fstring_copy_construct_caller),
                qualification_adapter_role::fstring_copy_construct};
        }
        if (entry.kind == registration_kind::object_behaviour &&
            entry.convention == call_convention::cdecl_object_first &&
            entry.behaviour == object_behaviour::destruct && entry.declaration == "void f()") {
            return qualification_adapter{asFUNCTION(qualification_fstring_destruct),
                asFunctionCaller(&qualification_fstring_destruct_caller),
                qualification_adapter_role::fstring_destruct};
        }
        if (entry.kind == registration_kind::object_method &&
            entry.convention == call_convention::thiscall &&
            entry.declaration == "FString& opAssign(const FString& Other)") {
            return qualification_adapter{asMETHOD(qualification_fstring, assign),
                asFunctionCaller(&qualification_fstring_assign_caller),
                qualification_adapter_role::fstring_assign};
        }
        break;
    case qualification_runtime_kind::none:
        break;
    }
    return std::nullopt;
}

bool record_qualification_adapter(
    registry_runtime::impl& runtime,
    const qualification_adapter& adapter,
    asCScriptFunction& function) {
    if (!runtime.qualification_roles.emplace(adapter.role, &function).second) return false;
    runtime.qualification_functions.insert(&function);
    return true;
}

std::string qualified_type_name(const asITypeInfo& type) {
    const char* name = type.GetName();
    const char* name_space = type.GetNamespace();
    std::string result;
    if (name_space != nullptr && *name_space != '\0') {
        result.assign(name_space);
        result.append("::");
    }
    if (name != nullptr) result.append(name);
    return result;
}

void remember_type_operations(
    registry_runtime::impl& runtime,
    asITypeInfo& type,
    const type_operations& operations) {
    const std::lock_guard<std::mutex> lock(runtime.operations_mutex);
    runtime.type_operations_by_pointer.emplace(&type, operations);
    runtime.type_operations_by_name.emplace(qualified_type_name(type), operations);
}

struct resolved_type_operations {
    bool valid = false;
    type_operations_kind kind = type_operations_kind::unavailable;
    fixed_type_operations fixed;
};

std::optional<std::size_t> primitive_index(const int type_id) noexcept {
    switch (type_id & asTYPEID_MASK_SEQNBR) {
    case asTYPEID_BOOL: return 0U;
    case asTYPEID_INT8: return 1U;
    case asTYPEID_INT16: return 2U;
    case asTYPEID_INT32: return 3U;
    case asTYPEID_INT64: return 4U;
    case asTYPEID_UINT8: return 5U;
    case asTYPEID_UINT16: return 6U;
    case asTYPEID_UINT32: return 7U;
    case asTYPEID_UINT64: return 8U;
    case asTYPEID_FLOAT32: return 9U;
    case asTYPEID_FLOAT64: return 10U;
    default: return std::nullopt;
    }
}

resolved_type_operations resolve_type_operations(
    registry_runtime::impl& runtime,
    asIScriptEngine& engine,
    const int type_id) {
    if (const std::optional<std::size_t> index = primitive_index(type_id)) {
        return {true, type_operations_kind::fixed, runtime.primitive_operations[*index]};
    }

    asITypeInfo* type = engine.GetTypeInfoById(type_id);
    if (type == nullptr) return {};
    if (template_operation_record* record = existing_template_record(runtime, *type)) {
        if (record->kind == type_operations_kind::fixed) {
            return {record->valid, record->kind, record->first};
        }
        fixed_type_operations container;
        container.can_be_template_subtype = false;
        return {record->valid, record->kind, container};
    }
    const asDWORD flags = type->GetFlags();
    if ((flags & asOBJ_SCRIPT_OBJECT) != 0U) {
        const auto category = runtime.dynamic_script_categories.find(type);
        fixed_type_operations operations;
        operations.can_create_property = true;
        operations.never_requires_gc = false;
        // FAngelscriptObjectType inherits the target default RequiresProperty=false even for
        // script reference classes. asOBJ_REF describes AngelScript storage, not the independent
        // ClassGenerator policy virtual.
        operations.requires_property = false;
        operations.can_be_template_subtype = true;
        operations.can_construct = true;
        operations.can_destruct = true;
        operations.can_copy = true;
        operations.can_compare = true;
        if ((flags & asOBJ_VALUE) == 0U) {
            operations.need_construct = true;
            operations.need_destruct = false;
            operations.need_copy = false;
            operations.can_hash_value = true;
            operations.value_size = static_cast<std::uint32_t>(sizeof(void*));
            operations.value_alignment = static_cast<std::uint32_t>(alignof(void*));
            operations.is_object_pointer = true;
        } else {
            if (category != runtime.dynamic_script_categories.end() &&
                category->second == dynamic_script_type_category::delegate) {
                operations = runtime.dynamic_script_operations.delegate;
                return {true, type_operations_kind::fixed, operations};
            }
            if (category != runtime.dynamic_script_categories.end() &&
                category->second == dynamic_script_type_category::multicast_delegate) {
                operations = runtime.dynamic_script_operations.multicast_delegate;
                return {true, type_operations_kind::fixed, operations};
            }
            if (category == runtime.dynamic_script_categories.end() &&
                type->GetUserData() != nullptr) {
                return {};
            }
            const int size = type->GetSize();
            const int alignment = static_cast<asCTypeInfo*>(type)->alignment;
            if (size < 0 || alignment <= 0) return {};
            operations.need_construct = true;
            operations.need_destruct = true;
            operations.need_copy = true;
            operations.can_hash_value = false;
            operations.value_size = static_cast<std::uint32_t>(size);
            operations.value_alignment = static_cast<std::uint32_t>(alignment);
        }
        return {valid_fixed_operations(operations), type_operations_kind::fixed, operations};
    }
    if ((flags & asOBJ_ENUM) != 0U && type->GetModule() != nullptr) {
        fixed_type_operations operations;
        operations.can_create_property = true;
        operations.never_requires_gc = false;
        operations.requires_property = false;
        operations.can_be_template_subtype = true;
        operations.can_construct = true;
        operations.need_construct = false;
        operations.can_destruct = true;
        operations.need_destruct = false;
        operations.can_copy = true;
        operations.need_copy = true;
        operations.can_compare = true;
        operations.can_hash_value = true;
        operations.value_size = 1U;
        operations.value_alignment = 1U;
        return {true, type_operations_kind::fixed, operations};
    }

    const auto by_pointer = runtime.type_operations_by_pointer.find(type);
    const type_operations* captured = by_pointer == runtime.type_operations_by_pointer.end()
        ? nullptr : &by_pointer->second;
    if (captured == nullptr) {
        const auto by_name = runtime.type_operations_by_name.find(qualified_type_name(*type));
        if (by_name != runtime.type_operations_by_name.end()) captured = &by_name->second;
    }
    if (captured == nullptr || captured->kind == type_operations_kind::unavailable) return {};
    if (captured->kind == type_operations_kind::fixed) {
        return {true, captured->kind, captured->fixed};
    }

    fixed_type_operations container;
    container.can_be_template_subtype = false;
    return {true, captured->kind, container};
}

template_operation_record* existing_template_record(
    registry_runtime::impl& runtime,
    asITypeInfo& type) {
    const auto* pointer = static_cast<const template_operation_record*>(type.GetUserData());
    if (pointer == nullptr || runtime.template_record_pointers.count(pointer) == 0U) return nullptr;
    return const_cast<template_operation_record*>(pointer);
}

template_operation_record& cache_template_record(
    registry_runtime::impl& runtime,
    asITypeInfo& type,
    std::unique_ptr<template_operation_record> record) {
    template_operation_record* pointer = record.get();
    runtime.template_record_pointers.insert(pointer);
    runtime.template_records.push_back(std::move(record));
    type.SetUserData(pointer);
    return *pointer;
}

void set_error(asCString* error, const char* message) {
    if (error != nullptr) *error = message;
}

bool validate_class_template(asITypeInfo* type, asCString* error) {
    if (type == nullptr || type->GetSubTypeCount() != 1U) return false;
    asITypeInfo* subtype = type->GetSubType(0U);
    if (subtype == nullptr || (subtype->GetFlags() & asOBJ_VALUE) != 0U) {
        set_error(error, "Subtype must be a class type");
        return false;
    }
    registry_runtime::impl* runtime = find_runtime(type->GetEngine());
    if (runtime == nullptr) return false;
    const std::lock_guard<std::mutex> lock(runtime->operations_mutex);
    if (template_operation_record* existing = existing_template_record(*runtime, *type)) {
        return existing->valid;
    }
    const int size = type->GetSize();
    const int alignment = static_cast<asCTypeInfo*>(type)->alignment;
    if (size <= 0 || alignment <= 0) {
        set_error(error, "Class wrapper has an invalid value layout");
        return false;
    }
    auto record = std::make_unique<template_operation_record>();
    record->kind = type_operations_kind::fixed;
    record->first.can_be_template_subtype = true;
    record->first.can_construct = true;
    record->first.need_construct = true;
    record->first.can_destruct = true;
    record->first.need_destruct = true;
    record->first.can_copy = true;
    record->first.need_copy = true;
    record->first.can_compare = true;
    record->first.can_hash_value = true;
    record->first.value_size = static_cast<std::uint32_t>(size);
    record->first.value_alignment = static_cast<std::uint32_t>(alignment);
    record->valid = true;
    return cache_template_record(*runtime, *type, std::move(record)).valid;
}

asIScriptFunction* find_hash_function(asITypeInfo* type) {
    if (type == nullptr) return nullptr;
    return type->GetMethodByDecl("uint32 Hash() const");
}

asIScriptFunction* find_compare_function(
    asITypeInfo* type,
    const bool is_object_pointer) {
    if (type == nullptr || type->GetName() == nullptr) return nullptr;
    std::string declaration = "int opCmp(";
    if (!is_object_pointer) declaration.append("const ");
    declaration.append(type->GetName());
    if (!is_object_pointer) declaration.push_back('&');
    declaration.append(" Other) const");
    return type->GetMethodByDecl(declaration.c_str());
}

bool validate_array_template(asITypeInfo* type, asCString* error) {
    if (type == nullptr || type->GetSubTypeCount() != 1U) return false;
    if (asITypeInfo* subtype = type->GetSubType(0U);
        subtype != nullptr && (subtype->GetFlags() & asOBJ_TEMPLATE_SUBTYPE) != 0U) {
        return true;
    }
    registry_runtime::impl* runtime = find_runtime(type->GetEngine());
    if (runtime == nullptr) return false;
    const std::lock_guard<std::mutex> lock(runtime->operations_mutex);
    if (template_operation_record* existing = existing_template_record(*runtime, *type)) {
        return existing->valid;
    }
    const resolved_type_operations subtype = resolve_type_operations(
        *runtime, *type->GetEngine(), type->GetSubTypeId(0U));
    if (!subtype.fixed.can_be_template_subtype) {
        set_error(error, "Containers cannot be nested in other containers");
        return false;
    }

    auto record = std::make_unique<template_operation_record>();
    record->kind = type_operations_kind::t_array;
    record->first = subtype.fixed;
    template_operation_record& cached = cache_template_record(*runtime, *type, std::move(record));
    if (!subtype.valid) {
        set_error(error, "Subtype could not be found");
        return false;
    }
    if (!(subtype.fixed.can_construct && subtype.fixed.can_destruct && subtype.fixed.can_copy)) {
        set_error(error, "Subtype cannot be constructed or copied");
        return false;
    }
    cached.valid = subtype.fixed.value_size > 0U;
    if (!cached.valid) {
        set_error(error, "Subtype is an empty struct");
        return false;
    }
    cached.compare_function = find_compare_function(
        type->GetSubType(0U), subtype.fixed.is_object_pointer);
    if (cached.compare_function != nullptr) {
        static_cast<asCScriptFunction*>(cached.compare_function)->isInUse = true;
    }
    return true;
}

bool validate_set_template(asITypeInfo* type, asCString* error) {
    if (type == nullptr || type->GetSubTypeCount() != 1U) return false;
    registry_runtime::impl* runtime = find_runtime(type->GetEngine());
    if (runtime == nullptr) return false;
    const std::lock_guard<std::mutex> lock(runtime->operations_mutex);
    if (template_operation_record* existing = existing_template_record(*runtime, *type)) {
        return existing->valid;
    }
    const resolved_type_operations subtype = resolve_type_operations(
        *runtime, *type->GetEngine(), type->GetSubTypeId(0U));
    if (!subtype.fixed.can_be_template_subtype) {
        set_error(error, "Containers cannot be nested in other containers");
        return false;
    }
    auto record = std::make_unique<template_operation_record>();
    record->kind = type_operations_kind::t_set;
    record->first = subtype.fixed;
    if (!subtype.fixed.can_hash_value) {
        record->hash_function = find_hash_function(type->GetSubType(0U));
    }
    const bool can_hash = subtype.fixed.can_hash_value || record->hash_function != nullptr;
    record->valid = subtype.valid && subtype.fixed.can_construct &&
        subtype.fixed.can_destruct && subtype.fixed.can_copy &&
        subtype.fixed.can_compare && can_hash;
    template_operation_record& cached = cache_template_record(*runtime, *type, std::move(record));
    if (!cached.valid) {
        set_error(error, can_hash ? "Key type does not have a hash function defined" :
            "Subtype cannot be constructed or copied");
    }
    return cached.valid;
}

bool validate_map_template(asITypeInfo* type, asCString* error) {
    if (type == nullptr || type->GetSubTypeCount() != 2U) return false;
    registry_runtime::impl* runtime = find_runtime(type->GetEngine());
    if (runtime == nullptr) return false;
    const std::lock_guard<std::mutex> lock(runtime->operations_mutex);
    if (template_operation_record* existing = existing_template_record(*runtime, *type)) {
        return existing->valid;
    }
    const resolved_type_operations key = resolve_type_operations(
        *runtime, *type->GetEngine(), type->GetSubTypeId(0U));
    const resolved_type_operations value = resolve_type_operations(
        *runtime, *type->GetEngine(), type->GetSubTypeId(1U));
    if (!key.fixed.can_be_template_subtype || !value.fixed.can_be_template_subtype) {
        set_error(error, "Containers cannot be nested in other containers");
        return false;
    }
    auto record = std::make_unique<template_operation_record>();
    record->kind = type_operations_kind::t_map;
    record->first = key.fixed;
    record->second = value.fixed;
    if (!key.fixed.can_hash_value) {
        record->hash_function = find_hash_function(type->GetSubType(0U));
    }
    const bool can_hash = key.fixed.can_hash_value || record->hash_function != nullptr;
    record->valid = key.valid && value.valid && key.fixed.can_construct &&
        key.fixed.can_destruct && key.fixed.can_copy && key.fixed.can_compare && can_hash &&
        value.fixed.can_construct && value.fixed.can_destruct && value.fixed.can_copy;
    template_operation_record& cached = cache_template_record(*runtime, *type, std::move(record));
    if (!cached.valid) {
        set_error(error, can_hash ? "Key type does not have a hash function defined" :
            "Subtype cannot be constructed or copied");
    }
    return cached.valid;
}

bool validate_optional_template(asITypeInfo* type, asCString* error) {
    if (type == nullptr || type->GetSubTypeCount() != 1U) return false;
    registry_runtime::impl* runtime = find_runtime(type->GetEngine());
    if (runtime == nullptr) return false;
    const std::lock_guard<std::mutex> lock(runtime->operations_mutex);
    if (template_operation_record* existing = existing_template_record(*runtime, *type)) {
        return existing->valid;
    }
    const resolved_type_operations subtype = resolve_type_operations(
        *runtime, *type->GetEngine(), type->GetSubTypeId(0U));
    if (!subtype.fixed.can_be_template_subtype) {
        set_error(error, "Containers cannot be nested in other containers");
        return false;
    }
    auto record = std::make_unique<template_operation_record>();
    record->kind = type_operations_kind::t_optional;
    record->first = subtype.fixed;
    record->valid = subtype.valid && subtype.fixed.can_construct &&
        subtype.fixed.can_destruct && subtype.fixed.can_copy;
    return cache_template_record(*runtime, *type, std::move(record)).valid;
}

void* auxiliary_pointer(registry_runtime::impl& runtime, const std::uint32_t id) {
    const auto iterator = runtime.objects.find(id);
    return iterator == runtime.objects.end() ? nullptr : iterator->second.get();
}

std::uint32_t public_type_id(
    asCScriptEngine& engine,
    asCTypeInfo& type,
    const char* const declaration) {
    if (declaration != nullptr) {
        const int declared_id = engine.GetTypeIdByDecl(declaration);
        if (declared_id >= 0) return static_cast<std::uint32_t>(declared_id);
    }
    return static_cast<std::uint32_t>(type.GetTypeId());
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
            remember_type_operations(runtime, *type, entry.operations);
            maps.type_declarations.emplace(
                entry.logical_id, callable_type_declaration(entry.declaration));
            actual.engine_id = public_type_id(
                engine, *type, maps.type_declarations.at(entry.logical_id).c_str());
            if (qualification_object_type_matches(runtime, entry)) {
                runtime.qualification_type_ids.insert(entry.logical_id);
                runtime.qualification_types.insert(type);
            }
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
            remember_type_operations(runtime, *type, entry.operations);
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
            actual.owner_engine_type_id = public_type_id(engine, *owner, owner_decl());
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
            actual.owner_engine_type_id = public_type_id(engine, *owner, owner_decl());
            actual.index = before;
        } else if (code >= 0) {
            return fail(registry_replay_phase::register_entry, entry.ordinal, "object property count did not advance exactly once");
        }
        break;
    }
    case registration_kind::object_method: {
        const auto adapter = owner_decl() == nullptr ? std::optional<qualification_adapter>{} :
            qualification_adapter_for(runtime, entry, owner_decl());
        code = engine.RegisterObjectMethod(
            owner_decl(), entry.declaration.c_str(),
            adapter.has_value() ? adapter->function : callable_stub(entry.convention),
            to_call_convention(entry.convention),
            adapter.has_value() ? adapter->caller : asFunctionCaller{}, auxiliary(),
            static_cast<int>(entry.composite_offset), entry.is_composite_indirect,
            entry.accessor_type);
        if (code >= 0) {
            auto* function = static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered object method is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, function);
            if (adapter.has_value() && !record_qualification_adapter(runtime, *adapter, *function)) {
                return fail(registry_replay_phase::register_entry, entry.ordinal,
                    "qualification adapter role is duplicated");
            }
            actual.engine_id = static_cast<std::uint32_t>(code);
            actual.owner_engine_type_id = public_type_id(engine, *owner, owner_decl());
        }
        break;
    }
    case registration_kind::object_behaviour: {
        const auto adapter = entry.behaviour == object_behaviour::template_callback ||
            owner_decl() == nullptr ? std::optional<qualification_adapter>{} :
            qualification_adapter_for(runtime, entry, owner_decl());
        const asSFuncPtr function = adapter.has_value() ? adapter->function :
            (entry.behaviour == object_behaviour::template_callback
                ? template_callback(entry.validation_adapter)
                : callable_stub(entry.convention));
        code = engine.RegisterObjectBehaviour(
            owner_decl(), to_behaviour(entry.behaviour), entry.declaration.c_str(), function,
            to_call_convention(entry.convention),
            adapter.has_value() ? adapter->caller : asFunctionCaller{}, auxiliary(),
            static_cast<int>(entry.composite_offset), entry.is_composite_indirect);
        if (code >= 0) {
            auto* reflected_function =
                static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (reflected_function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered object behaviour is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, reflected_function);
            if (adapter.has_value() &&
                !record_qualification_adapter(runtime, *adapter, *reflected_function)) {
                return fail(registry_replay_phase::register_entry, entry.ordinal,
                    "qualification adapter role is duplicated");
            }
            actual.engine_id = static_cast<std::uint32_t>(code);
            actual.owner_engine_type_id = public_type_id(engine, *owner, owner_decl());
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
    case registration_kind::global_function: {
        const auto adapter = qualification_adapter_for(runtime, entry, {});
        code = engine.RegisterGlobalFunction(
            entry.declaration.c_str(),
            adapter.has_value() ? adapter->function : callable_stub(entry.convention),
            to_call_convention(entry.convention),
            adapter.has_value() ? adapter->caller : asFunctionCaller{}, auxiliary());
        if (code >= 0) {
            auto* function = static_cast<asCScriptFunction*>(engine.GetFunctionById(code));
            if (function == nullptr) {
                return fail(registry_replay_phase::register_entry, entry.ordinal, "registered global function is not reflectable");
            }
            maps.functions.emplace(entry.logical_id, function);
            if (adapter.has_value() && !record_qualification_adapter(runtime, *adapter, *function)) {
                return fail(registry_replay_phase::register_entry, entry.ordinal,
                    "qualification adapter role is duplicated");
            }
            actual.engine_id = static_cast<std::uint32_t>(code);
        }
        break;
    }
    case registration_kind::enum_type:
        code = engine.RegisterEnum(entry.declaration.c_str());
        if (code >= 0) {
            auto* type = static_cast<asCTypeInfo*>(engine.GetTypeInfoById(code));
            if (type == nullptr) return fail(registry_replay_phase::register_entry, entry.ordinal, "registered enum is not reflectable");
            maps.types.emplace(entry.logical_id, type);
            remember_type_operations(runtime, *type, entry.operations);
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
            remember_type_operations(runtime, *type, entry.operations);
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
    case registration_kind::string_factory: {
        const bool qualification_fstring =
            runtime.qualification_kind == qualification_runtime_kind::fstring_roundtrip &&
            entry.context.name_space.empty() && entry.declaration == "FString" &&
            runtime.qualification_type_ids.size() == 1U;
        runtime.strings = std::make_unique<string_pool>(qualification_fstring);
        code = engine.RegisterStringFactory(entry.declaration.c_str(), runtime.strings.get());
        if (code >= 0) {
            actual.installed = true;
            runtime.qualification_string_factory = qualification_fstring;
        }
        break;
    }
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

registry_replay_result apply_registration_time_state(
    const post_bind_state& state,
    const std::size_t ordinal,
    replay_maps& maps) {
    switch (state.kind) {
    case post_bind_state_kind::object_type: {
        asCObjectType* type = object_type(maps, state.logical_id);
        if (type == nullptr) {
            return fail(
                registry_replay_phase::apply_post_bind_state, ordinal,
                "object type registration-time state is invalid");
        }
        // Template instances copy these switches when their declaration is first
        // encountered. Waiting until all registrations have finished leaves those
        // already-created instances with stale conversion/subtype semantics.
        type->hasImplicitConstructors = state.has_implicit_constructors;
        type->acceptValueSubType = state.accepts_value_subtype;
        type->acceptRefSubType = state.accepts_reference_subtype;
        break;
    }
    case post_bind_state_kind::function:
        // Generated template functions copy traits and system-function metadata.
        // Restore both before a later registration can instantiate the template.
        return apply_final_state(state, ordinal, maps);
    case post_bind_state_kind::object_property:
    case post_bind_state_kind::global_property:
        break;
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

registry_runtime::impl::~impl() { unregister_runtime(*this); }

bool classify_dynamic_script_type(
    registry_runtime& runtime,
    asITypeInfo& type,
    const dynamic_script_type_category category) {
    if (runtime.impl_ == nullptr || !runtime.impl_->bound ||
        runtime.impl_->engine != type.GetEngine() ||
        static_cast<unsigned>(category) >
            static_cast<unsigned>(dynamic_script_type_category::multicast_delegate)) {
        return false;
    }
    const asDWORD flags = type.GetFlags();
    if ((flags & asOBJ_SCRIPT_OBJECT) == 0U || (flags & asOBJ_VALUE) == 0U) {
        return false;
    }
    const std::lock_guard<std::mutex> lock(runtime.impl_->operations_mutex);
    runtime.impl_->dynamic_script_categories.insert_or_assign(&type, category);
    return true;
}

registry_runtime::registry_runtime() : impl_(std::make_unique<impl>()) {}
registry_runtime::~registry_runtime() = default;
registry_runtime::registry_runtime(registry_runtime&&) noexcept = default;
registry_runtime& registry_runtime::operator=(registry_runtime&&) noexcept = default;

bool registry_runtime::resolve_class_generator_type_capabilities(
    asIScriptEngine& engine,
    const int type_id,
    class_generator_type_capabilities& output,
    std::string& detail) {
    if (impl_ == nullptr || type_id <= 0) {
        detail = "class-generator type capability request is invalid";
        return false;
    }
    if (primitive_index(type_id).has_value()) {
        output = {true, false, false};
        return true;
    }
    asITypeInfo* const core_type = engine.GetTypeInfoById(type_id);
    if (core_type == nullptr) {
        detail = "class-generator type identity is unavailable";
        return false;
    }
    const asDWORD core_flags = core_type->GetFlags();
    if ((core_flags & asOBJ_SCRIPT_OBJECT) != 0U) {
        output = {true, false, false};
        return true;
    }
    if ((core_flags & asOBJ_ENUM) != 0U && core_type->GetModule() != nullptr) {
        output = {true, false, false};
        return true;
    }
    if (!impl_->bound || impl_->engine != &engine) {
        detail = "registered class-generator type capability request is not bound to this registry";
        return false;
    }
    const std::lock_guard<std::mutex> lock(impl_->operations_mutex);
    const resolved_type_operations operations =
        resolve_type_operations(*impl_, engine, type_id);
    if (!operations.valid) {
        detail = "class-generator type capabilities are unavailable";
        return false;
    }

    class_generator_type_capabilities resolved;
    if (operations.kind == type_operations_kind::fixed) {
        resolved.can_create_property = operations.fixed.can_create_property;
        resolved.never_requires_gc = operations.fixed.never_requires_gc;
        resolved.requires_property = operations.fixed.requires_property;
        output = resolved;
        return true;
    }

    template_operation_record* const record =
        existing_template_record(*impl_, *core_type);
    if (record == nullptr || record->kind != operations.kind || !record->valid) {
        detail = "class-generator container capabilities have no validated subtype record";
        return false;
    }
    switch (operations.kind) {
    case type_operations_kind::t_array:
    case type_operations_kind::t_optional:
        resolved.can_create_property = record->first.can_create_property;
        break;
    case type_operations_kind::t_set:
        resolved.can_create_property = record->first.can_create_property &&
            record->first.can_hash_value;
        break;
    case type_operations_kind::t_map:
        resolved.can_create_property = record->first.can_create_property &&
            record->first.can_hash_value && record->second.can_create_property;
        break;
    case type_operations_kind::unavailable:
    case type_operations_kind::fixed:
        detail = "class-generator type capability kind is inconsistent";
        return false;
    }
    // The four captured container implementations inherit both target defaults.
    resolved.never_requires_gc = false;
    resolved.requires_property = false;
    output = resolved;
    return true;
}

bool registry_runtime::configure_qualification_runtime(
    const qualification_runtime_kind kind,
    const std::vector<std::string>& static_names,
    const std::vector<std::string>& static_name_comparison_identities,
    std::string& detail) {
    if (impl_ == nullptr || impl_->bound || kind == qualification_runtime_kind::none ||
        static_names.size() != static_name_comparison_identities.size() ||
        static_names.size() > 1'000'000U) {
        detail = "qualification runtime configuration is invalid or already bound";
        return false;
    }
    impl_->qualification_kind = kind;
    impl_->qualification_static_name_identities = static_name_comparison_identities;
    impl_->qualification_static_names.assign(static_names.size(), {});
    std::unordered_map<std::string, std::uint32_t> comparison_ids;
    comparison_ids.reserve(static_names.size());
    std::uint32_t next_id = 1U;
    for (std::size_t index = 0U; index < static_names.size(); ++index) {
        const std::string& identity = static_name_comparison_identities[index];
        if (identity.empty()) continue;
        auto [iterator, inserted] = comparison_ids.emplace(identity, next_id);
        if (inserted) {
            if (next_id == std::numeric_limits<std::uint32_t>::max()) {
                detail = "qualification FName comparison identity space is exhausted";
                return false;
            }
            ++next_id;
        }
        impl_->qualification_static_names[index].comparison_index = iterator->second;
    }
    return true;
}

bool registry_runtime::qualification_runtime_ready(std::string& detail) const {
    if (impl_ == nullptr || !impl_->bound ||
        impl_->qualification_kind == qualification_runtime_kind::none ||
        impl_->qualification_type_ids.empty()) {
        detail = "qualification runtime is not bound to a captured host type";
        return false;
    }
    const auto require = [&](const qualification_adapter_role role,
                             const asEFirstParamMetaData metadata) {
        const auto found = impl_->qualification_roles.find(role);
        if (found == impl_->qualification_roles.end() || found->second == nullptr) return false;
        const auto* const function = static_cast<const asCScriptFunction*>(found->second);
        return function->sysFuncIntf != nullptr &&
            function->sysFuncIntf->passFirstParamMetaData == metadata;
    };
    bool ready = false;
    switch (impl_->qualification_kind) {
    case qualification_runtime_kind::t_array_int32:
        ready = require(qualification_adapter_role::array_construct, asEFirstParamMetaData::None) &&
            require(qualification_adapter_role::array_destruct, asEFirstParamMetaData::ScriptObjectType) &&
            require(qualification_adapter_role::array_assign, asEFirstParamMetaData::ScriptObjectType) &&
            require(qualification_adapter_role::array_set_num, asEFirstParamMetaData::ScriptObjectType);
        break;
    case qualification_runtime_kind::fname_equivalence:
        ready = require(qualification_adapter_role::fname_equals, asEFirstParamMetaData::None) &&
            require(qualification_adapter_role::fname_static_name, asEFirstParamMetaData::None);
        break;
    case qualification_runtime_kind::fstring_roundtrip:
        ready = impl_->qualification_string_factory &&
            require(qualification_adapter_role::fstring_construct, asEFirstParamMetaData::None) &&
            require(qualification_adapter_role::fstring_copy_construct, asEFirstParamMetaData::None) &&
            require(qualification_adapter_role::fstring_destruct, asEFirstParamMetaData::None) &&
            require(qualification_adapter_role::fstring_assign, asEFirstParamMetaData::None);
        break;
    case qualification_runtime_kind::none:
        break;
    }
    if (!ready) detail = "captured registry lacks the exact donor ABI adapters required by the qualification invoke";
    return ready;
}

bool registry_runtime::prepare_qualification_runtime(
    asIScriptEngine& interface, std::string& detail) {
    if (impl_ == nullptr || !impl_->bound || impl_->engine != &interface ||
        impl_->qualification_kind == qualification_runtime_kind::none) {
        detail = "qualification runtime is not bound to the requested engine";
        return false;
    }
    auto& engine = static_cast<asCScriptEngine&>(interface);
    std::vector<asCScriptFunction*> additions;
    for (asUINT index = 0U; index < engine.scriptFunctions.GetLength(); ++index) {
        asCScriptFunction* const candidate = engine.scriptFunctions[index];
        if (candidate == nullptr || candidate->funcType != asFUNC_SYSTEM ||
            candidate->sysFuncIntf == nullptr || candidate->objectType == nullptr ||
            impl_->qualification_functions.count(candidate) != 0U ||
            !qualification_object_type_allowed(candidate->objectType)) {
            continue;
        }
        const asCScriptFunction* donor = nullptr;
        for (const asIScriptFunction* const allowed : impl_->qualification_functions) {
            const auto* const captured = static_cast<const asCScriptFunction*>(allowed);
            if (captured == nullptr || captured->sysFuncIntf == nullptr ||
                captured->name != candidate->name ||
                captured->parameterTypes.GetLength() != candidate->parameterTypes.GetLength() ||
                captured->sysFuncIntf->callConv != candidate->sysFuncIntf->callConv ||
                captured->sysFuncIntf->caller.type != candidate->sysFuncIntf->caller.type) {
                continue;
            }
            const bool same_caller = captured->sysFuncIntf->caller.type == 1
                ? captured->sysFuncIntf->caller.FunctionCaller ==
                    candidate->sysFuncIntf->caller.FunctionCaller
                : captured->sysFuncIntf->caller.type == 2 &&
                    captured->sysFuncIntf->caller.MethodCaller ==
                        candidate->sysFuncIntf->caller.MethodCaller;
            if (!same_caller) continue;
            if (donor != nullptr) {
                detail = "qualification adapter caller resolves to multiple donor roles";
                return false;
            }
            donor = captured;
        }
        if (donor == nullptr) continue;
        const asEFirstParamMetaData donor_metadata =
            donor->sysFuncIntf->passFirstParamMetaData;
        if (candidate->sysFuncIntf->passFirstParamMetaData != donor_metadata) {
            if (impl_->qualification_kind != qualification_runtime_kind::t_array_int32 ||
                candidate->sysFuncIntf->passFirstParamMetaData !=
                    asEFirstParamMetaData::None ||
                donor_metadata != asEFirstParamMetaData::ScriptObjectType) {
                detail = "qualification adapter clone has incompatible first-parameter metadata";
                return false;
            }
            // The donor fork creates concrete TArray<T> method clones after registration and
            // drops ScriptObjectType metadata on this path. The function caller is still the
            // exact qualification adapter, so restore only that captured donor ABI field.
            candidate->sysFuncIntf->passFirstParamMetaData = donor_metadata;
        }
        additions.push_back(candidate);
    }
    impl_->qualification_functions.insert(additions.begin(), additions.end());
    return true;
}

bool registry_runtime::qualification_function_allowed(
    const asIScriptFunction* const function) const noexcept {
    if (impl_ == nullptr || function == nullptr) return false;
    if (impl_->qualification_functions.count(function) != 0U) return true;
    const auto* const candidate = static_cast<const asCScriptFunction*>(function);
    if (candidate->sysFuncIntf == nullptr || candidate->objectType == nullptr ||
        !qualification_object_type_allowed(candidate->objectType)) return false;
    return std::any_of(impl_->qualification_functions.begin(),
        impl_->qualification_functions.end(), [&](const asIScriptFunction* allowed) {
            const auto* const captured = static_cast<const asCScriptFunction*>(allowed);
            if (captured == nullptr || captured->sysFuncIntf == nullptr ||
                captured->sysFuncIntf->caller.type != candidate->sysFuncIntf->caller.type ||
                captured->sysFuncIntf->callConv != candidate->sysFuncIntf->callConv ||
                captured->sysFuncIntf->passFirstParamMetaData !=
                    candidate->sysFuncIntf->passFirstParamMetaData) return false;
            if (captured->sysFuncIntf->caller.type == 1) {
                return captured->sysFuncIntf->caller.FunctionCaller ==
                    candidate->sysFuncIntf->caller.FunctionCaller;
            }
            return captured->sysFuncIntf->caller.type == 2 &&
                captured->sysFuncIntf->caller.MethodCaller ==
                    candidate->sysFuncIntf->caller.MethodCaller;
        });
}

bool registry_runtime::qualification_object_type_allowed(
    const asITypeInfo* const type) const noexcept {
    if (impl_ == nullptr || type == nullptr) return false;
    if (impl_->qualification_types.count(type) != 0U) return true;
    if (impl_->qualification_kind != qualification_runtime_kind::t_array_int32 ||
        type->GetName() == nullptr || std::string_view(type->GetName()) != "TArray" ||
        type->GetSize() != sizeof(qualification_script_array) ||
        type->GetSubTypeCount() != 1U || type->GetSubTypeId(0U) != asTYPEID_INT32) {
        return false;
    }
    return true;
}

bool registry_runtime::qualification_global_address_allowed(
    const void* const address) const noexcept {
    return impl_ != nullptr && impl_->strings != nullptr &&
        impl_->strings->contains_qualification_value(address);
}

bool registry_runtime::qualification_instruction_allowed(
    asIScriptEngine& engine,
    const asDWORD* const instruction,
    const asBYTE opcode,
    std::string& detail) const {
    if (impl_ == nullptr || impl_->qualification_kind == qualification_runtime_kind::none ||
        instruction == nullptr) {
        detail = "qualification host instruction gate is not configured";
        return false;
    }
    const auto reject = [&]() {
        detail = "qualification invoke bytecode contains unauthorized opcode ";
        detail += asBCInfo[opcode].name;
        return false;
    };
    switch (opcode) {
    case asBC_CALLSYS:
    case asBC_Thiscall1: {
        const auto* const function = reinterpret_cast<const asIScriptFunction*>(
            asBC_PTRARG(instruction));
        if (qualification_function_allowed(function)) return true;
        reject();
        detail += " target=";
        if (function == nullptr) {
            detail += "<null>";
        } else {
            const char* const declaration = function->GetDeclaration(true, true, true);
            detail += declaration == nullptr ? "<missing-declaration>" : declaration;
            detail += " id=" + std::to_string(function->GetId());
            const asITypeInfo* const owner = function->GetObjectType();
            detail += " owner=";
            detail += owner == nullptr || owner->GetName() == nullptr
                ? "<global>" : owner->GetName();
            const auto* const candidate = static_cast<const asCScriptFunction*>(function);
            if (candidate->sysFuncIntf != nullptr) {
                detail += " abi=(caller=" +
                    std::to_string(candidate->sysFuncIntf->caller.type) + ",conv=" +
                    std::to_string(candidate->sysFuncIntf->callConv) + ",meta=" +
                    std::to_string(static_cast<int>(
                        candidate->sysFuncIntf->passFirstParamMetaData)) + ")";
            }
            detail += " allowed=[";
            bool first = true;
            for (const asIScriptFunction* const allowed : impl_->qualification_functions) {
                if (!first) detail += ';';
                first = false;
                const auto* const captured = static_cast<const asCScriptFunction*>(allowed);
                const char* const captured_declaration =
                    allowed == nullptr ? nullptr : allowed->GetDeclaration(true, true, true);
                detail += captured_declaration == nullptr
                    ? "<missing-declaration>" : captured_declaration;
                if (captured != nullptr && captured->sysFuncIntf != nullptr) {
                    const bool same_caller = candidate->sysFuncIntf != nullptr &&
                        captured->sysFuncIntf->caller.type ==
                            candidate->sysFuncIntf->caller.type &&
                        ((captured->sysFuncIntf->caller.type == 1 &&
                          captured->sysFuncIntf->caller.FunctionCaller ==
                              candidate->sysFuncIntf->caller.FunctionCaller) ||
                         (captured->sysFuncIntf->caller.type == 2 &&
                          captured->sysFuncIntf->caller.MethodCaller ==
                              candidate->sysFuncIntf->caller.MethodCaller));
                    detail += "(caller=" +
                        std::to_string(captured->sysFuncIntf->caller.type) + ",conv=" +
                        std::to_string(captured->sysFuncIntf->callConv) + ",meta=" +
                        std::to_string(static_cast<int>(
                            captured->sysFuncIntf->passFirstParamMetaData)) + ",same=" +
                        (same_caller ? "1" : "0") + ")";
                }
            }
            detail += ']';
        }
        return false;
    }
    case asBC_ALLOC: {
        const auto* const type = reinterpret_cast<const asITypeInfo*>(asBC_PTRARG(instruction));
        const int function_id = asBC_INTARG(instruction + AS_PTR_SIZE);
        return (qualification_object_type_allowed(type) && function_id != 0 &&
            qualification_function_allowed(engine.GetFunctionById(function_id))) || reject();
    }
    case asBC_FREE:
    case asBC_OBJTYPE:
        return qualification_object_type_allowed(
            reinterpret_cast<const asITypeInfo*>(asBC_PTRARG(instruction))) || reject();
    case asBC_TYPEID: {
        const int type_id = static_cast<int>(asBC_DWORDARG(instruction));
        return (type_id == asTYPEID_INT32 ||
            qualification_object_type_allowed(engine.GetTypeInfoById(type_id))) || reject();
    }
    case asBC_PGA:
        return qualification_global_address_allowed(
            reinterpret_cast<const void*>(asBC_PTRARG(instruction))) || reject();
    // No script/import/interface/function-pointer calls, arbitrary globals, raw string opcode,
    // list allocator, script-object lifecycle, object resolver or reference-debug callbacks.
    case asBC_PshGPtr: case asBC_PshG4: case asBC_LdGRdR4: case asBC_CALL:
    case asBC_STR: case asBC_CALLBND: case asBC_CpyVtoG4: case asBC_CpyGtoV4:
    case asBC_LDG: case asBC_SetG4: case asBC_CALLINTF: case asBC_CallPtr:
    case asBC_FuncPtr: case asBC_LoadThisR: case asBC_AllocMem:
    case asBC_SetListSize: case asBC_PshListElmnt: case asBC_SetListType:
    case asBC_FinConstruct: case asBC_DestructScript: case asBC_CopyScript:
    case asBC_ResolveObjectPtr: case asBC_TrackRef: case asBC_UntrackRef:
    case asBC_ValidateRef: case asBC_ThrowException:
        return reject();
    default:
        // All remaining instructions operate on the VM stack/registers/local storage. The
        // request is the exact sealed zero-argument corpus source; the only routes from that
        // closed VM state into host memory/callables are validated above.
        return true;
    }
}

qualification_runtime_kind registry_runtime::qualification_kind() const noexcept {
    return impl_ == nullptr ? qualification_runtime_kind::none : impl_->qualification_kind;
}

bool registry_runtime::read_qualification_tarray_int32(
    const void* const object,
    std::vector<std::int32_t>& values,
    std::string& detail) const {
    values.clear();
    if (qualification_kind() != qualification_runtime_kind::t_array_int32 || object == nullptr) {
        detail = "qualification TArray<int32> return object is unavailable";
        return false;
    }
    const auto& array = *static_cast<const qualification_script_array*>(object);
    if (array.count < 0 || array.count > qualification_max_array_values ||
        array.capacity < array.count || array.capacity > qualification_max_array_values ||
        (array.count != 0 && array.data == nullptr)) {
        detail = "qualification TArray<int32> return layout is invalid";
        return false;
    }
    const auto* data = static_cast<const std::int32_t*>(array.data);
    if (array.count != 0) values.assign(data, data + array.count);
    return true;
}

bool registry_runtime::read_qualification_fstring(
    const void* const object,
    std::string& value,
    std::string& detail) const {
    if (qualification_kind() != qualification_runtime_kind::fstring_roundtrip || object == nullptr ||
        !qualification_utf16_to_utf8(*static_cast<const qualification_fstring*>(object), value)) {
        detail = "qualification FString return layout/UTF-16 payload is invalid";
        return false;
    }
    return true;
}

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
    prepared->qualification_kind = runtime.impl_->qualification_kind;
    prepared->qualification_static_name_identities =
        runtime.impl_->qualification_static_name_identities;
    prepared->qualification_static_names = runtime.impl_->qualification_static_names;
    for (std::size_t index = 0U; index < profile.primitive_operations.size(); ++index) {
        prepared->primitive_operations[index] = profile.primitive_operations[index].operations;
    }
    prepared->dynamic_script_operations = profile.dynamic_script_operations;
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
    if (!register_runtime(engine, *prepared)) {
        return fail(
            registry_replay_phase::validate_profile, no_ordinal,
            "engine already has a standalone registry runtime");
    }

    for (const engine_property_setting& setting : profile.engine_properties) {
        const int code = engine.SetEngineProperty(to_engine_property(setting.property), setting.value);
        if (code < 0 || engine.GetEngineProperty(to_engine_property(setting.property)) != setting.value) {
            return fail(registry_replay_phase::apply_engine_properties, setting.ordinal, "engine property did not apply exactly", code);
        }
    }

    replay_maps maps;
    std::unordered_map<std::uint32_t, std::pair<const post_bind_state*, std::size_t>>
        registration_time_type_states;
    std::unordered_map<std::uint32_t, std::pair<const post_bind_state*, std::size_t>>
        registration_time_function_states;
    registration_time_type_states.reserve(profile.final_states.size());
    registration_time_function_states.reserve(profile.final_states.size());
    for (std::size_t index = 0U; index < profile.final_states.size(); ++index) {
        const post_bind_state& state = profile.final_states[index];
        if (state.kind == post_bind_state_kind::object_type) {
            registration_time_type_states.emplace(
                state.logical_id, std::make_pair(&state, index));
        } else if (state.kind == post_bind_state_kind::function) {
            registration_time_function_states.emplace(
                state.logical_id, std::make_pair(&state, index));
        }
    }
    for (std::size_t index = 0U; index < profile.registrations.size(); ++index) {
        const registration_entry& entry = profile.registrations[index];
        result = apply_context(engine, entry);
        if (!result.succeeded()) return result;
        registration_result actual;
        result = register_one(engine, profile, entry, *prepared, maps, actual);
        if (!result.succeeded()) return result;
        if (!same_result(actual, profile.expected_results[index])) {
            const registration_result& expected = profile.expected_results[index];
            return fail(
                registry_replay_phase::verify_registration_result, index,
                "registration result differs from captured post-bind identity: actual(kind=" +
                    std::to_string(static_cast<unsigned>(actual.kind)) + ",engine_id=" +
                    std::to_string(actual.engine_id) + ",owner_engine_type_id=" +
                    std::to_string(actual.owner_engine_type_id) + ",index=" +
                    std::to_string(actual.index) + ",installed=" +
                    std::to_string(actual.installed) + "), expected(kind=" +
                    std::to_string(static_cast<unsigned>(expected.kind)) + ",engine_id=" +
                    std::to_string(expected.engine_id) + ",owner_engine_type_id=" +
                    std::to_string(expected.owner_engine_type_id) + ",index=" +
                    std::to_string(expected.index) + ",installed=" +
                    std::to_string(expected.installed) + ")");
        }
        const auto apply_registration_time = [&](const auto& states) {
            const auto state = states.find(entry.logical_id);
            if (state == states.end()) return registry_replay_result{};
            return apply_registration_time_state(
                *state->second.first, state->second.second, maps);
        };
        switch (entry.kind) {
        case registration_kind::object_type:
        case registration_kind::interface_type:
            result = apply_registration_time(registration_time_type_states);
            break;
        case registration_kind::interface_method:
        case registration_kind::object_method:
        case registration_kind::object_behaviour:
        case registration_kind::global_function:
            result = apply_registration_time(registration_time_function_states);
            break;
        case registration_kind::object_property:
        case registration_kind::global_property:
        case registration_kind::enum_type:
        case registration_kind::enum_value:
        case registration_kind::funcdef:
        case registration_kind::typedef_type:
        case registration_kind::string_factory:
        case registration_kind::default_array_type:
            result = {};
            break;
        }
        if (!result.succeeded()) return result;
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
