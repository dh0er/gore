#include "gore_as_standalone/precompiled_engine.hpp"
#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/precompiled_metadata.hpp"

#include "as_builder.h"
#include "as_bytecode.h"
#include "as_datatype.h"
#include "as_module.h"
#include "as_objecttype.h"
#include "as_property.h"
#include "as_scriptengine.h"
#include "as_scriptfunction.h"
#include "as_tokendef.h"

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <exception>
#include <limits>
#include <memory>
#include <new>
#include <string_view>
#include <tuple>
#include <unordered_map>
#include <unordered_set>
#include <utility>

namespace gore::as::standalone::precompiled {
namespace {

constexpr std::size_t kNoModule = static_cast<std::size_t>(-1);

bool is_editor_only_module_name(const std::string_view module_name) noexcept {
    return module_name.compare(0U, 7U, "Editor.") == 0 ||
        module_name.find(".Editor.") != std::string_view::npos;
}

engine_bridge_result failure(
    const engine_bridge_phase phase,
    const std::size_t module_index,
    std::string detail,
    const int code = asNOT_SUPPORTED) {
    return engine_bridge_result{code, phase, module_index, std::move(detail)};
}

[[nodiscard]] bool plain_module_name(const map_string& key, std::string& output) {
    if (key.utf16 || key.payload.empty()) {
        return false;
    }
    for (const std::uint8_t value : key.payload) {
        if (value < 0x20U || value > 0x7eU || value == '\\' || value == '/') {
            return false;
        }
    }
    output.assign(key.payload.begin(), key.payload.end());
    return true;
}

[[nodiscard]] bool is_primitive(const data_type& type) noexcept {
    if (type.type_info != 0 || type.is_auto || type.is_object_handle ||
        type.if_handle_then_const) {
        return false;
    }
    switch (static_cast<eTokenType>(type.token_type)) {
    case ttBool:
    case ttInt:
    case ttInt8:
    case ttInt16:
    case ttInt64:
    case ttUInt:
    case ttUInt8:
    case ttUInt16:
    case ttUInt64:
    case ttFloat:
    case ttFloat32:
    case ttFloat64:
    case ttVoid: return true;
    default: return false;
    }
}

[[nodiscard]] bool valid_data_type_shape(const data_type& type) noexcept {
    if (type.is_auto) {
        return type.type_info == 0;
    }
    if (type.type_info != 0) {
        return true;
    }
    return is_primitive(type);
}

[[nodiscard]] bool is_storage_type(const data_type& type) noexcept {
    return valid_data_type_shape(type) && !type.is_auto &&
           type.token_type != static_cast<std::int32_t>(ttVoid) && !type.is_reference;
}

[[nodiscard]] bool is_parameter_type(const data_type& type) noexcept {
    if (type.type_info == 0 && type.token_type == static_cast<std::int32_t>(ttQuestion)) {
        return type.is_reference && !type.is_auto && !type.is_object_handle &&
            !type.if_handle_then_const;
    }
    return valid_data_type_shape(type) && !type.is_auto &&
           type.token_type != static_cast<std::int32_t>(ttVoid);
}

[[nodiscard]] bool is_type_modifier(const std::int32_t value) noexcept {
    return value >= asTM_NONE && value <= asTM_CONST;
}

[[nodiscard]] bool engine_name(const archive_string& value) noexcept {
    return !value.bytes.empty() && value.bytes.find('\0') == std::string::npos;
}

[[nodiscard]] bool empty_preprocessor_data(const precompiled_class& type) noexcept {
    return !type.is_in_preprocessor && type.super_class.bytes.empty() &&
           type.code_super_class.bytes.empty() && !type.super_is_code_class &&
           !type.abstract && !type.transient && !type.hide_dropdown &&
           !type.default_to_instanced && !type.edit_inline_new &&
           !type.is_deprecated_class && type.config_name.bytes.empty() &&
           type.static_class_global_variable_name.bytes.empty() && !type.placeable &&
           type.metadata_specifiers.empty() && type.metadata_values.empty() &&
           type.compose_onto_class_name.bytes.empty();
}

bool valid_metadata(
    const std::vector<archive_string>& specifiers,
    const std::vector<archive_string>& values) noexcept {
    if (specifiers.size() != values.size()) return false;
    for (std::size_t index = 0U; index < specifiers.size(); ++index) {
        if (!engine_name(specifiers[index]) ||
            values[index].bytes.find('\0') != std::string::npos) {
            return false;
        }
    }
    return true;
}

bool validate_function_shape(
    const precompiled_function& function,
    std::string& detail,
    bool allow_unreal_metadata = false);

bool validate_class_shape(
    const precompiled_class& type,
    std::string& detail,
    const bool allow_unreal_metadata = false,
    const bool allow_shadow_type = false) {
    if (!engine_name(type.class_name) ||
        type.name_space.bytes.find('\0') != std::string::npos) {
        detail = "class name/namespace must be non-empty engine strings without NUL bytes";
        return false;
    }
    const asDWORD flags = static_cast<asDWORD>(type.flags);
    if ((flags & asOBJ_SCRIPT_OBJECT) == 0U ||
        ((flags & asOBJ_REF) != 0U) == ((flags & asOBJ_VALUE) != 0U) ||
        (flags & asOBJ_TEMPLATE) != 0U) {
        detail = "checkpoint bridge accepts only concrete script class/struct flags";
        return false;
    }
    if (type.shadow_type != 0 && !allow_shadow_type) {
        detail = "checkpoint bridge does not yet recreate UObject shadow layouts";
        return false;
    }
    if (!type.behaviour_references.empty() && type.behaviour_references.size() != 7U) {
        detail = "class behaviour table must be empty or contain exactly seven references";
        return false;
    }
    if ((flags & asOBJ_VALUE) == 0U) {
        for (const std::int32_t method_index : type.method_table) {
            if (method_index < -1 ||
                (method_index >= 0 &&
                 static_cast<std::size_t>(method_index) >= type.methods.size())) {
                detail = "class virtual method table contains an invalid method index";
                return false;
            }
            if (method_index == -1 && type.derived_from == 0) {
                detail = "class without a base type cannot inherit an empty virtual slot";
                return false;
            }
        }
    }
    if (type.behaviour_functions.size() != type.behaviour_function_types.size()) {
        detail = "class behaviour function/type arrays do not have identical lengths";
        return false;
    }
    for (const std::int32_t behaviour : type.behaviour_function_types) {
        if (behaviour != asBEHAVE_DESTRUCT) {
            detail = "fork cache only supports serialized destructor behaviour functions";
            return false;
        }
    }
    for (const precompiled_function& function : type.methods) {
        if (!validate_function_shape(function, detail, allow_unreal_metadata)) {
            return false;
        }
    }
    for (const precompiled_function& function : type.constructors) {
        if (!validate_function_shape(function, detail, allow_unreal_metadata)) {
            return false;
        }
    }
    for (const precompiled_function& function : type.behaviour_functions) {
        if (!validate_function_shape(function, detail, allow_unreal_metadata)) {
            return false;
        }
    }
    if (!allow_unreal_metadata && !empty_preprocessor_data(type)) {
        detail = "checkpoint bridge does not yet replay class preprocessor metadata";
        return false;
    }
    if (allow_unreal_metadata &&
        (type.super_class.bytes.find('\0') != std::string::npos ||
         type.code_super_class.bytes.find('\0') != std::string::npos ||
         type.config_name.bytes.find('\0') != std::string::npos ||
         type.static_class_global_variable_name.bytes.find('\0') != std::string::npos ||
         type.compose_onto_class_name.bytes.find('\0') != std::string::npos ||
         !valid_metadata(type.metadata_specifiers, type.metadata_values))) {
        detail = "class preprocessor metadata is structurally invalid";
        return false;
    }
    for (const precompiled_property& property : type.properties) {
        if (!engine_name(property.name) || !is_storage_type(property.type)) {
            detail = "class properties must have named non-void storage data types";
            return false;
        }
        if (!allow_unreal_metadata &&
            (property.is_unreal_property || !property.metadata_specifiers.empty() ||
            !property.metadata_values.empty() || property.blueprint_readable ||
            property.blueprint_writable || property.edit_const ||
            property.editable_on_defaults || property.editable_on_instance ||
            property.instanced_reference || property.persistent_instance ||
            property.advanced_display || property.transient || property.replicated ||
            property.replication_condition != 0 || property.skip_replication ||
            property.skip_serialization || property.save_game || property.rep_notify ||
            property.config || property.interp || property.asset_registry_searchable)) {
            detail = "checkpoint bridge does not yet replay Unreal property metadata";
            return false;
        }
        if (allow_unreal_metadata &&
            !valid_metadata(property.metadata_specifiers, property.metadata_values)) {
            detail = "property metadata arrays are invalid";
            return false;
        }
    }
    return true;
}

bool validate_function_shape(
    const precompiled_function& function,
    std::string& detail,
    const bool allow_unreal_metadata) {
    const std::size_t parameter_count = function.parameter_types.size();
    if (function.parameter_names.size() != parameter_count ||
        function.parameter_flags.size() != parameter_count ||
        function.parameter_default_args.size() != parameter_count) {
        detail = "function parameter arrays do not have identical lengths";
        return false;
    }
    if (function.function_name.bytes.find('\0') != std::string::npos ||
        function.name_space.bytes.find('\0') != std::string::npos ||
        std::any_of(
            function.parameter_names.begin(), function.parameter_names.end(),
            [](const archive_string& value) {
                return value.bytes.find('\0') != std::string::npos;
            }) ||
        std::any_of(
            function.parameter_default_args.begin(), function.parameter_default_args.end(),
            [](const archive_string& value) {
                return value.bytes.find('\0') != std::string::npos;
            })) {
        detail = "function identity, parameter name, or default argument contains an embedded NUL";
        return false;
    }
    if (!valid_data_type_shape(function.return_type) ||
        std::any_of(
            function.parameter_types.begin(), function.parameter_types.end(),
            [](const data_type& type) { return !is_parameter_type(type); }) ||
        std::any_of(
            function.parameter_flags.begin(), function.parameter_flags.end(),
            [](const std::int32_t value) { return !is_type_modifier(value); })) {
        detail = "function contains an invalid return or parameter data type";
        return false;
    }
    if (function.object_variable_types.size() != function.object_variable_positions.size()) {
        detail = "function object-local arrays do not have identical lengths";
        return false;
    }
    if (!function.byte_code_references.empty()) {
        detail = "checkpoint bridge does not accept legacy ByteCodeReferences entries";
        return false;
    }
    if (function.variable_space < 0 || function.stack_needed < 0) {
        detail = "compiled function variable-space and stack requirements must be non-negative";
        return false;
    }
    if (function.stack_needed < function.variable_space) {
        detail = "compiled function stack requirement is smaller than its variable space";
        return false;
    }
    std::unordered_set<std::int32_t> object_positions;
    for (const std::int32_t position : function.object_variable_positions) {
        if (position <= 0 || position > function.variable_space ||
            !object_positions.emplace(position).second) {
            detail = "function object-local position is outside VariableSpace or duplicated";
            return false;
        }
    }
    if (function.object_variables_on_heap < 0 ||
        static_cast<std::size_t>(function.object_variables_on_heap) >
            function.object_variable_positions.size()) {
        detail = "function object-local heap prefix is outside the object-local array";
        return false;
    }
    if (function.variable_info_program_positions.size() != function.variable_info_offsets.size() ||
        function.variable_info_program_positions.size() != function.variable_info_options.size()) {
        detail = "function variable-info arrays do not have identical lengths";
        return false;
    }
    std::int32_t previous_program_position = -1;
    std::size_t block_depth = 0U;
    for (std::size_t index = 0U;
         index < function.variable_info_program_positions.size(); ++index) {
        const std::int32_t program_position =
            function.variable_info_program_positions[index];
        const std::int32_t variable_offset = function.variable_info_offsets[index];
        const std::int32_t option = function.variable_info_options[index];
        if (program_position < 0 ||
            static_cast<std::size_t>(program_position) > function.byte_code.size() ||
            program_position < previous_program_position) {
            detail = "function variable-info program positions are out of bytecode range or order";
            return false;
        }
        previous_program_position = program_position;
        switch (option) {
        case asOBJ_UNINIT:
        case asOBJ_INIT:
            if (object_positions.count(variable_offset) == 0U) {
                detail = "function variable-info object offset is absent from object locals";
                return false;
            }
            break;
        case asBLOCK_BEGIN:
            if (variable_offset != 0) {
                detail = "function variable-info block begin carries a nonzero offset";
                return false;
            }
            ++block_depth;
            break;
        case asBLOCK_END:
            if (variable_offset != 0 || block_depth == 0U) {
                detail = "function variable-info block end is unmatched or carries an offset";
                return false;
            }
            --block_depth;
            break;
        default:
            detail = "function variable-info option is unknown";
            return false;
        }
    }
    if (block_depth != 0U) {
        detail = "function variable-info blocks are not balanced";
        return false;
    }
    if ((function.line_numbers.size() & 1U) != 0U) {
        detail = "function line-number array does not contain position/value pairs";
        return false;
    }
    std::int32_t previous_line_position = -1;
    for (std::size_t index = 0U; index < function.line_numbers.size(); index += 2U) {
        const std::int32_t position = function.line_numbers[index];
        if (position < 0 || static_cast<std::size_t>(position) > function.byte_code.size() ||
            position < previous_line_position) {
            detail = "function line-number positions are out of bytecode range or order";
            return false;
        }
        previous_line_position = position;
    }
    if (function.is_unreal_function && !allow_unreal_metadata) {
        detail = "checkpoint bridge does not yet attach UFunction descriptors";
        return false;
    }
    if (allow_unreal_metadata &&
        (function.unreal_function_name.bytes.find('\0') != std::string::npos ||
         !valid_metadata(
             function.metadata_specifiers, function.metadata_values))) {
        detail = "function Unreal metadata is structurally invalid";
        return false;
    }

    std::size_t offset = 0U;
    while (offset < function.byte_code.size()) {
        const auto opcode = static_cast<unsigned char>(function.byte_code[offset] & 0xff);
        if (opcode > static_cast<unsigned char>(asBC_MAXBYTECODE)) {
            detail = "function bytecode contains an unknown opcode";
            return false;
        }
        const int instruction_size = asBCTypeSize[asBCInfo[opcode].type];
        if (instruction_size <= 0 ||
            static_cast<std::size_t>(instruction_size) > function.byte_code.size() - offset) {
            detail = "function bytecode instruction extends beyond its array";
            return false;
        }
        if (static_cast<asEBCInstr>(opcode) == asBC_STR) {
            detail = "fork precompiled bytecode must not contain asBC_STR";
            return false;
        }
        offset += static_cast<std::size_t>(instruction_size);
    }
    if (offset != function.byte_code.size()) {
        detail = "function bytecode does not end on an instruction boundary";
        return false;
    }
    return true;
}

bool validate_module_shape(
    const std::pair<map_string, precompiled_module>& entry,
    std::string& module_name,
    std::string& detail,
    const bool allow_mixed_metadata = false) {
    const bool plain_key = plain_module_name(entry.first, module_name);
    if (!plain_key || entry.second.module_name.bytes != module_name) {
        detail = "module TMap key and inner ModuleName must be the same non-empty ASCII name"
            " (key=" + (plain_key ? module_name : std::string("<non-ASCII-or-empty>")) +
            ", key_utf16=" + std::to_string(entry.first.utf16) +
            ", key_bytes=" + std::to_string(entry.first.payload.size()) +
            ", inner=" + entry.second.module_name.bytes + ")";
        return false;
    }
    const precompiled_module& module = entry.second;
    if ((!allow_mixed_metadata && !module.statics_class_name.bytes.empty()) ||
        (!allow_mixed_metadata && !module.post_init_functions.empty()) ||
        (!allow_mixed_metadata &&
         (!module.declared_events.empty() || !module.declared_delegates.empty()))) {
        detail = "checkpoint bridge does not yet replay requested module metadata";
        return false;
    }
    std::unordered_set<std::string> declared_delegate_names;
    for (const archive_string& declared : module.declared_events) {
        if (!engine_name(declared) ||
            !declared_delegate_names.emplace(declared.bytes).second) {
            detail = "declared event/delegate names must be non-empty and unique";
            return false;
        }
    }
    for (const archive_string& declared : module.declared_delegates) {
        if (!engine_name(declared) ||
            !declared_delegate_names.emplace(declared.bytes).second) {
            detail = "declared event/delegate names must be non-empty and unique";
            return false;
        }
    }
    if (allow_mixed_metadata &&
        (module.statics_class_name.bytes.find('\0') != std::string::npos ||
         std::any_of(
             module.post_init_functions.begin(), module.post_init_functions.end(),
             [](const archive_string& name) { return !engine_name(name); }))) {
        detail = "module statics/post-init metadata is structurally invalid";
        return false;
    }
    for (const archive_string& imported : module.imported_modules) {
        if (!engine_name(imported)) {
            detail = "imported module names must be non-empty engine strings";
            return false;
        }
    }
    for (const function_import& imported : module.function_imports) {
        const function_signature& signature = imported.signature;
        if (!engine_name(imported.imported_from_module) || !engine_name(signature.name) ||
            signature.name_space.bytes.find('\0') != std::string::npos ||
            signature.parameter_types.size() != signature.parameter_flags.size() ||
            signature.parameter_types.size() != signature.parameter_default_args.size() ||
            !valid_data_type_shape(signature.return_type) ||
            std::any_of(
                signature.parameter_types.begin(), signature.parameter_types.end(),
                [](const data_type& type) { return !is_parameter_type(type); }) ||
            std::any_of(
                signature.parameter_flags.begin(), signature.parameter_flags.end(),
                [](const std::int32_t value) { return !is_type_modifier(value); })) {
            detail = "function import contains an invalid module/signature shape";
            return false;
        }
    }
    for (const precompiled_class& type : module.classes) {
        if (!validate_class_shape(
                type, detail, allow_mixed_metadata, allow_mixed_metadata)) {
            return false;
        }
    }
    for (const precompiled_enum& enumeration : module.enums) {
        if (enumeration.names.size() != enumeration.values.size()) {
            detail = "enum name/value arrays do not have identical lengths";
            return false;
        }
    }
    for (const precompiled_global& global : module.global_variables) {
        if (!is_storage_type(global.type)) {
            detail = "global contains an invalid non-storage data type";
            return false;
        }
        if (global.is_default_init && global.is_pure_constant) {
            detail = "global cannot be both default-initialized and a pure constant";
            return false;
        }
        if (global.has_init_function &&
            (global.is_default_init || global.is_pure_constant ||
             !validate_function_shape(
                 global.init_function, detail, allow_mixed_metadata))) {
            if (detail.empty()) {
                detail = "global initializer flags are contradictory";
            }
            return false;
        }
    }
    for (const precompiled_function& function : module.functions) {
        if (!validate_function_shape(function, detail, allow_mixed_metadata)) {
            return false;
        }
    }
    return true;
}

engine_bridge_result preflight_cache(
    asCScriptEngine& engine,
    const cache& input,
    std::vector<std::string>& module_names,
    const bool allow_mixed_metadata = false) {
    std::unordered_set<std::string> unique_names;
    std::unordered_set<std::uint32_t> declared_function_ids;
    module_names.clear();
    module_names.reserve(input.modules.size());
    for (std::size_t index = 0U; index < input.modules.size(); ++index) {
        std::string name;
        std::string detail;
        if (!validate_module_shape(
                input.modules[index], name, detail, allow_mixed_metadata)) {
            return failure(engine_bridge_phase::preflight, index, std::move(detail));
        }
        if (!unique_names.emplace(name).second) {
            return failure(
                engine_bridge_phase::preflight, index, "duplicate module name in cache");
        }
        if (engine.GetModule(name.c_str(), false) != nullptr) {
            return failure(
                engine_bridge_phase::preflight, index,
                "target engine already contains a module with this name", asALREADY_REGISTERED);
        }
        const auto record_function = [&](const precompiled_function& function) {
            return function.id != 0U && declared_function_ids.emplace(function.id).second;
        };
        bool function_ids_valid = true;
        for (const precompiled_function& function : input.modules[index].second.functions) {
            function_ids_valid = function_ids_valid && record_function(function);
        }
        for (const precompiled_class& type : input.modules[index].second.classes) {
            for (const precompiled_function& function : type.methods) {
                function_ids_valid = function_ids_valid && record_function(function);
            }
            for (const precompiled_function& function : type.constructors) {
                function_ids_valid = function_ids_valid && record_function(function);
            }
            for (const precompiled_function& function : type.behaviour_functions) {
                function_ids_valid = function_ids_valid && record_function(function);
            }
        }
        for (const precompiled_global& global : input.modules[index].second.global_variables) {
            if (global.has_init_function) {
                function_ids_valid =
                    function_ids_valid && record_function(global.init_function);
            }
        }
        if (!function_ids_valid) {
            return failure(
                engine_bridge_phase::preflight, index,
                "serialized function ids must be nonzero and unique across the cache");
        }
        module_names.push_back(std::move(name));
    }

    std::unordered_set<std::int64_t> pointer_keys;
    std::unordered_set<std::int64_t> type_keys;
    for (const auto& entry : input.type_references) {
        if (entry.first == 0 || !pointer_keys.emplace(entry.first).second ||
            !type_keys.emplace(entry.first).second || !engine_name(entry.second.name) ||
            entry.second.module.bytes.find('\0') != std::string::npos ||
            entry.second.name_space.bytes.find('\0') != std::string::npos ||
            std::any_of(
                entry.second.sub_types.begin(), entry.second.sub_types.end(),
                [](const data_type& type) { return !valid_data_type_shape(type); })) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "invalid or colliding type-reference table entry");
        }
    }
    std::unordered_set<std::int32_t> type_id_keys;
    for (const auto& entry : input.type_id_reference_to_pointer) {
        if (entry.first <= asTYPEID_LAST_PRIMITIVE ||
            !type_id_keys.emplace(entry.first).second ||
            type_keys.find(entry.second) == type_keys.end()) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "type-id table does not map uniquely to a saved type reference");
        }
    }

    std::unordered_set<std::int64_t> function_keys;
    for (std::size_t function_index = 0U;
         function_index < input.function_references.size(); ++function_index) {
        const auto& entry = input.function_references[function_index];
        const function_reference& reference = entry.second;
        std::string reason;
        if (entry.first == 0) reason = "zero pointer key";
        else if (pointer_keys.count(entry.first) != 0U) reason = "pointer key collides with another reference table";
        else if (function_keys.count(entry.first) != 0U) reason = "duplicate function pointer key";
        else if (!engine_name(reference.name)) reason = "empty or NUL-containing function name";
        else if (reference.module.bytes.find('\0') != std::string::npos) reason = "NUL-containing module name";
        else if (reference.name_space.bytes.find('\0') != std::string::npos) reason = "NUL-containing namespace";
        else if (!valid_data_type_shape(reference.return_type)) reason = "invalid return type";
        else {
            const auto invalid_parameter = std::find_if(
                reference.parameter_types.begin(), reference.parameter_types.end(),
                [](const data_type& type) { return !is_parameter_type(type); });
            if (invalid_parameter != reference.parameter_types.end()) {
                reason = "invalid parameter type at index " + std::to_string(
                    static_cast<std::size_t>(invalid_parameter - reference.parameter_types.begin())) +
                    " (type_info=" + std::to_string(invalid_parameter->type_info) +
                    ", token_type=" + std::to_string(invalid_parameter->token_type) +
                    ", is_reference=" + std::to_string(invalid_parameter->is_reference) +
                    ", is_object_handle=" + std::to_string(invalid_parameter->is_object_handle) +
                    ", is_auto=" + std::to_string(invalid_parameter->is_auto) +
                    ", if_handle_then_const=" +
                    std::to_string(invalid_parameter->if_handle_then_const) + ")";
            } else if (reference.is_method && reference.object_type == 0) {
                reason = "method has no object type";
            }
        }
        if (!reason.empty()) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "invalid or colliding function-reference table entry (index=" +
                    std::to_string(function_index) + ", key=" +
                    std::to_string(entry.first) + ", name=" + reference.name.bytes +
                    ", module=" + reference.module.bytes + ", reason=" + reason + ")");
        }
        pointer_keys.emplace(entry.first);
        function_keys.emplace(entry.first);
    }
    std::unordered_set<std::int32_t> function_id_keys;
    for (const auto& entry : input.function_id_reference_to_pointer) {
        if (entry.first == 0 || !function_id_keys.emplace(entry.first).second ||
            function_keys.find(entry.second) == function_keys.end()) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "function-id table does not map uniquely to a saved function reference");
        }
    }

    for (const auto& entry : input.global_references) {
        const global_reference& reference = entry.second;
        if (entry.first == 0 || !pointer_keys.emplace(entry.first).second ||
            reference.name.bytes.find('\0') != std::string::npos ||
            (!reference.is_string && !engine_name(reference.name)) ||
            (reference.is_string &&
             (!reference.module.bytes.empty() ||
              !reference.name_space.bytes.empty() || engine.stringFactory == nullptr)) ||
            reference.module.bytes.find('\0') != std::string::npos ||
            reference.name_space.bytes.find('\0') != std::string::npos) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "invalid, colliding, or unavailable global/string reference");
        }
    }
    std::unordered_set<std::int64_t> property_keys;
    for (const auto& entry : input.property_references) {
        const std::int32_t base_type_id = entry.second.old_type_id &
            static_cast<std::int32_t>(asTYPEID_MASK_SEQNBR | asTYPEID_MASK_OBJECT);
        if (entry.first == 0 || !property_keys.emplace(entry.first).second ||
            !engine_name(entry.second.name) ||
            (base_type_id > asTYPEID_LAST_PRIMITIVE &&
             type_id_keys.find(base_type_id) == type_id_keys.end())) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "invalid property-reference table entry");
        }
    }
    if (std::any_of(
            input.static_names.begin(), input.static_names.end(),
            [](const archive_string& value) {
                return value.bytes.find('\0') != std::string::npos;
            })) {
        return failure(
            engine_bridge_phase::preflight, kNoModule,
            "static-name table contains an embedded NUL byte");
    }
    return {};
}

asSNameSpace* name_space(asCScriptEngine& engine, const archive_string& value) {
    return engine.AddNameSpace(value.bytes.c_str());
}

class reference_resolver final {
public:
    reference_resolver(asCScriptEngine& engine, const cache& input) : engine_(engine) {
        for (const auto& entry : input.type_references) {
            type_references_.emplace(entry.first, &entry.second);
        }
        for (const auto& entry : input.type_id_reference_to_pointer) {
            type_ids_.emplace(entry.first, entry.second);
        }
        for (const auto& entry : input.function_references) {
            function_references_.emplace(entry.first, &entry.second);
        }
        for (const auto& entry : input.function_id_reference_to_pointer) {
            function_ids_.emplace(entry.first, entry.second);
        }
        for (const auto& entry : input.global_references) {
            global_references_.emplace(entry.first, &entry.second);
        }
        for (const auto& entry : input.property_references) {
            property_references_.emplace(entry.first, &entry.second);
        }
    }

    bool create_data_type(
        const data_type& input,
        asCDataType& output,
        const bool add_ref,
        std::string& detail) {
        if (input.is_auto) {
            output = asCDataType::CreateAuto(input.is_object_const);
            if (input.is_object_handle) {
                output.MakeHandle(true);
                output.MakeReadOnly(input.is_const_handle);
            }
            if (input.is_reference) {
                output.MakeReference(true);
            }
            return true;
        }
        if (input.type_info != 0) {
            asCTypeInfo* type = nullptr;
            if (!get_type_info(input.type_info, type, false, detail) || type == nullptr) {
                return false;
            }
            if (input.is_object_handle) {
                output = asCDataType::CreateObjectHandle(type, input.is_const_handle);
                output.MakeHandleToConst(input.is_object_const);
            } else {
                output = asCDataType::CreateType(type, input.is_object_const);
            }
            if (input.is_reference) {
                output.MakeReference(true);
            }
            if (input.if_handle_then_const) {
                output.SetIfHandleThenConst(true);
            }
            if (add_ref) {
                type->AddRefInternal();
            }
            return true;
        }
        if (input.token_type == static_cast<std::int32_t>(ttQuestion) &&
            is_parameter_type(input)) {
            output = asCDataType::CreatePrimitive(ttQuestion, input.is_object_const);
            output.MakeReference(true);
            return true;
        }
        if (!is_primitive(input)) {
            detail = "data type has neither a valid primitive token nor a saved type reference";
            return false;
        }
        output = asCDataType::CreatePrimitive(
            static_cast<eTokenType>(input.token_type), input.is_object_const);
        if (input.is_reference) {
            output.MakeReference(true);
        }
        return true;
    }

    bool get_type_info(
        const std::int64_t old_reference,
        asCTypeInfo*& output,
        const bool add_ref,
        std::string& detail) {
        if (old_reference == 0) {
            output = nullptr;
            return true;
        }
        const auto cached = type_cache_.find(old_reference);
        if (cached != type_cache_.end()) {
            output = cached->second;
            if (add_ref) {
                output->AddRefInternal();
            }
            return true;
        }
        const auto found_reference = type_references_.find(old_reference);
        if (found_reference == type_references_.end()) {
            detail = "bytecode/data type references an absent saved type pointer";
            return false;
        }
        const type_reference& reference = *found_reference->second;
        asCTypeInfo* found = nullptr;
        asSNameSpace* const ns = name_space(engine_, reference.name_space);
        if (!reference.name_space.bytes.empty() && reference.module.bytes == "$__T__") {
            auto* base = static_cast<asCObjectType*>(
                engine_.GetTypeInfo(reference.name_space.bytes.c_str(), engine_.defaultNamespace));
            if (base != nullptr) {
                for (asUINT index = 0U; index < base->templateSubTypes.GetLength(); ++index) {
                    asCTypeInfo* subtype = base->templateSubTypes[index].GetTypeInfo();
                    if (subtype != nullptr && subtype->name == reference.name.bytes.c_str()) {
                        found = subtype;
                        break;
                    }
                }
            }
        } else if (!reference.module.bytes.empty() && reference.sub_types.empty()) {
            auto* module = static_cast<asCModule*>(
                engine_.GetModule(reference.module.bytes.c_str(), false));
            if (module != nullptr) {
                found = module->GetType(reference.name.bytes.c_str(), ns);
            }
        } else {
            found = static_cast<asCTypeInfo*>(
                engine_.GetTypeInfo(reference.name.bytes.c_str(), ns));
            if (found == nullptr && reference.name.bytes == "$obj") {
                found = &engine_.scriptTypeBehaviours;
            } else if (found == nullptr && reference.name.bytes == "$func") {
                found = &engine_.functionBehaviours;
            }
        }
        if (found == nullptr) {
            detail = "saved type reference could not be resolved by module/name/namespace"
                " (pointer=" + std::to_string(old_reference) +
                ", name=" + reference.name.bytes +
                ", module=" + reference.module.bytes +
                ", namespace=" + reference.name_space.bytes +
                ", subtypes=" + std::to_string(reference.sub_types.size()) + ")";
            return false;
        }
        if (!reference.sub_types.empty()) {
            auto* template_base = CastToObjectType(found);
            if (template_base == nullptr) {
                detail = "saved type reference supplies subtypes for a non-template object";
                return false;
            }
            asCArray<asCDataType> subtypes;
            subtypes.SetLength(static_cast<asUINT>(reference.sub_types.size()));
            for (asUINT index = 0U; index < subtypes.GetLength(); ++index) {
                if (!create_data_type(reference.sub_types[index], subtypes[index], false, detail)) {
                    return false;
                }
            }
            asCObjectType* instance =
                engine_.GetTemplateInstanceType(template_base, subtypes, nullptr);
            if (instance == nullptr) {
                detail = "engine refused to instantiate a saved template type";
                return false;
            }
            instance->AddRef();
            found = instance;
        }
        found->GetTypeId();
        type_cache_.emplace(old_reference, found);
        output = found;
        if (add_ref) {
            output->AddRefInternal();
        }
        return true;
    }

    bool get_type_id(
        const std::int64_t old_reference,
        int& output,
        const bool add_ref,
        std::string& detail) {
        if (old_reference == 0 || old_reference <= asTYPEID_LAST_PRIMITIVE) {
            output = static_cast<int>(old_reference);
            return true;
        }
        const auto raw = static_cast<asDWORD>(old_reference);
        const asDWORD flags = raw & ~(asTYPEID_MASK_SEQNBR | asTYPEID_MASK_OBJECT);
        const auto base_id = static_cast<std::int32_t>(
            raw & (asTYPEID_MASK_SEQNBR | asTYPEID_MASK_OBJECT));
        const auto pointer = type_ids_.find(base_id);
        if (pointer == type_ids_.end()) {
            detail = "saved type id has no type-pointer mapping";
            return false;
        }
        asCTypeInfo* type = nullptr;
        if (!get_type_info(pointer->second, type, add_ref, detail) || type == nullptr) {
            return false;
        }
        output = type->GetTypeId() | static_cast<int>(flags);
        return true;
    }

    bool get_function_id(
        const std::int64_t old_id,
        int& output,
        const bool add_ref,
        std::string& detail) {
        if (old_id == 0) {
            output = 0;
            return true;
        }
        if (old_id < std::numeric_limits<std::int32_t>::min() ||
            old_id > std::numeric_limits<std::int32_t>::max()) {
            detail = "saved function id does not fit the fork's 32-bit id space";
            return false;
        }
        const auto pointer = function_ids_.find(static_cast<std::int32_t>(old_id));
        if (pointer == function_ids_.end()) {
            detail = "saved function id has no function-pointer mapping";
            return false;
        }
        asCScriptFunction* function = nullptr;
        if (!get_function(pointer->second, function, add_ref, true, detail) ||
            function == nullptr) {
            return false;
        }
        output = function->GetId();
        return true;
    }

    void stage_function_bytecode(
        asCScriptFunction& function,
        const std::vector<std::int32_t>& bytecode) {
        pending_bytecode_.emplace(&function, &bytecode);
    }

    bool relocate_function(asCScriptFunction& function, std::string& detail) {
        if (function.scriptData == nullptr) {
            return true;
        }
        const auto pending = pending_bytecode_.find(&function);
        if (pending == pending_bytecode_.end()) {
            detail = "function bytecode was not staged for transactional relocation";
            return false;
        }
        std::vector<std::int32_t> relocated = *pending->second;
        asDWORD* instruction = reinterpret_cast<asDWORD*>(relocated.data());
        asDWORD* const end = instruction + relocated.size();
        while (instruction < end) {
            const auto opcode = static_cast<asEBCInstr>(*reinterpret_cast<asBYTE*>(instruction));
            switch (opcode) {
            case asBC_PshGPtr:
            case asBC_PshG4:
            case asBC_LdGRdR4:
            case asBC_CpyVtoG4:
            case asBC_CpyGtoV4:
            case asBC_LDG:
            case asBC_PGA:
            case asBC_SetG4: {
                void* address = nullptr;
                asCGlobalProperty* property = nullptr;
                if (!get_global(
                        static_cast<std::int64_t>(asBC_PTRARG(instruction)),
                        address, property, detail)) {
                    return false;
                }
                asBC_PTRARG(instruction) = reinterpret_cast<asPWORD>(address);
                break;
            }
            case asBC_CALL:
            case asBC_CALLBND:
            case asBC_CALLINTF: {
                int id = 0;
                if (!get_function_id(asBC_INTARG(instruction), id, false, detail)) {
                    return false;
                }
                asBC_INTARG(instruction) = id;
                break;
            }
            case asBC_CopyScript:
            case asBC_FinConstruct:
            case asBC_FREE:
            case asBC_DestructScript:
            case asBC_OBJTYPE: {
                asCTypeInfo* type = nullptr;
                if (!get_type_info(
                        static_cast<std::int64_t>(asBC_PTRARG(instruction)),
                        type, false, detail)) {
                    return false;
                }
                asBC_PTRARG(instruction) = reinterpret_cast<asPWORD>(type);
                break;
            }
            case asBC_COPY: {
                int id = 0;
                if (!get_type_id(asBC_INTARG(instruction), id, false, detail)) {
                    return false;
                }
                asBC_INTARG(instruction) = id;
                auto* type = static_cast<asCObjectType*>(engine_.GetTypeInfoById(id));
                if (type != nullptr) {
                    if (type->size > std::numeric_limits<short>::max()) {
                        detail = "relocated type size exceeds bytecode's signed-short operand";
                        return false;
                    }
                    asBC_SWORDARG0(instruction) = static_cast<short>(type->size);
                }
                break;
            }
            case asBC_STR:
                detail = "fork precompiled bytecode unexpectedly contains asBC_STR";
                return false;
            case asBC_CALLSYS:
            case asBC_FuncPtr:
            case asBC_Thiscall1: {
                asCScriptFunction* target = nullptr;
                if (!get_function(
                        static_cast<std::int64_t>(asBC_PTRARG(instruction)),
                        target, false, true, detail)) {
                    return false;
                }
                asBC_PTRARG(instruction) = reinterpret_cast<asPWORD>(target);
                break;
            }
            case asBC_ALLOC: {
                asCTypeInfo* type = nullptr;
                if (!get_type_info(
                        static_cast<std::int64_t>(asBC_PTRARG(instruction)),
                        type, false, detail)) {
                    return false;
                }
                asBC_PTRARG(instruction) = reinterpret_cast<asPWORD>(type);
                int id = 0;
                if (!get_function_id(
                        asBC_INTARG(instruction + AS_PTR_SIZE), id, false, detail)) {
                    return false;
                }
                asBC_INTARG(instruction + AS_PTR_SIZE) = id;
                break;
            }
            case asBC_TYPEID:
            case asBC_Cast: {
                int id = 0;
                if (!get_type_id(asBC_INTARG(instruction), id, false, detail)) {
                    return false;
                }
                asBC_INTARG(instruction) = id;
                break;
            }
            case asBC_SetListType: {
                int id = 0;
                if (!get_type_id(asBC_INTARG(instruction + 1), id, false, detail)) {
                    return false;
                }
                asBC_INTARG(instruction + 1) = id;
                break;
            }
            case asBC_ADDSi:
            case asBC_LoadThisR: {
                short offset = 0;
                int declaring_type_id = -1;
                if (!get_property_offset(
                        asBC_SWORDARG0(instruction), asBC_INTARG(instruction),
                        offset, declaring_type_id, detail)) {
                    return false;
                }
                asBC_SWORDARG0(instruction) = offset;
                // This operand is ignored by the VM after compilation. Keep the
                // relocated declaring type id so a mixed cached/source graph can
                // be exported again without recovering type identity heuristically.
                asBC_INTARG(instruction) = declaring_type_id;
                break;
            }
            case asBC_LoadRObjR:
            case asBC_LoadVObjR: {
                short offset = 0;
                int declaring_type_id = -1;
                auto* const old_type_id = reinterpret_cast<int*>(instruction + 2);
                if (!get_property_offset(
                        asBC_SWORDARG1(instruction), *old_type_id,
                        offset, declaring_type_id, detail)) {
                    return false;
                }
                asBC_SWORDARG1(instruction) = offset;
                *old_type_id = declaring_type_id;
                break;
            }
            default: break;
            }
            const int size = asBCTypeSize[asBCInfo[opcode].type];
            if (size <= 0 || size > end - instruction) {
                detail = "bytecode relocation lost the validated instruction boundary";
                return false;
            }
            instruction += size;
        }
        if (instruction != end) return false;
        function.scriptData->byteCode.SetLength(static_cast<asUINT>(relocated.size()));
        if (!relocated.empty()) {
            std::memcpy(
                function.scriptData->byteCode.AddressOf(), relocated.data(),
                relocated.size() * sizeof(relocated[0]));
        }
        function.AddReferences();
        pending_bytecode_.erase(pending);
        return true;
    }

private:
    bool signature_matches(
        const function_reference& reference,
        asCScriptFunction& function,
        std::string& detail,
        asCObjectType* const template_instance = nullptr) {
        if (function.IsReadOnly() != reference.is_const ||
            function.parameterTypes.GetLength() != reference.parameter_types.size()) {
            return false;
        }
        const bool substitute_template_types = template_instance != nullptr &&
            template_instance->templateBaseType != nullptr &&
            function.traits.GetTrait(asTRAIT_GENERIC_TEMPLATE_FUNCTION);
        asCDataType expected_return;
        if (!create_data_type(reference.return_type, expected_return, false, detail)) {
            return false;
        }
        const asCDataType actual_return = substitute_template_types
            ? engine_.DetermineTypeForTemplate(
                  function.returnType, template_instance->templateBaseType, template_instance)
            : function.returnType;
        if (actual_return != expected_return) {
            return false;
        }
        for (asUINT index = 0U; index < function.parameterTypes.GetLength(); ++index) {
            asCDataType expected;
            if (!create_data_type(reference.parameter_types[index], expected, false, detail)) {
                return false;
            }
            const asCDataType actual = substitute_template_types
                ? engine_.DetermineTypeForTemplate(
                      function.parameterTypes[index], template_instance->templateBaseType,
                      template_instance)
                : function.parameterTypes[index];
            if (actual != expected) {
                return false;
            }
        }
        return true;
    }

    bool get_function(
        const std::int64_t old_reference,
        asCScriptFunction*& output,
        const bool add_ref,
        const bool mark_in_use,
        std::string& detail) {
        if (old_reference == 0) {
            output = nullptr;
            return true;
        }
        const auto cached = function_cache_.find(old_reference);
        if (cached != function_cache_.end()) {
            output = cached->second;
            if (add_ref) {
                output->AddRefInternal();
            }
            if (mark_in_use) {
                output->isInUse = true;
            }
            return true;
        }
        const auto saved = function_references_.find(old_reference);
        if (saved == function_references_.end()) {
            detail = "bytecode references an absent saved function pointer";
            return false;
        }
        const function_reference& reference = *saved->second;
        asCScriptFunction* found = nullptr;
        std::string resolved_owner_name;
        std::string candidate_declarations;
        const auto remember_candidate = [&](asCScriptFunction* candidate) {
            if (candidate == nullptr || candidate_declarations.size() >= 2048U) {
                return;
            }
            if (!candidate_declarations.empty()) {
                candidate_declarations.append(" | ");
            }
            const char* declaration = candidate->GetDeclaration(true, true, true, true);
            candidate_declarations.append(declaration == nullptr ? "<no declaration>" : declaration);
        };
        if (reference.is_imported_decl) {
            auto* module = static_cast<asCModule*>(
                engine_.GetModule(reference.module.bytes.c_str(), false));
            if (module != nullptr) {
                for (asUINT index = 0U; index < module->bindInformations.GetLength(); ++index) {
                    asCScriptFunction* candidate =
                        module->bindInformations[index]->importedFunctionSignature;
                    if (candidate->name == reference.name.bytes.c_str() &&
                        signature_matches(reference, *candidate, detail)) {
                        found = candidate;
                        break;
                    }
                }
            }
        } else if (reference.is_method) {
            asCTypeInfo* type_info = nullptr;
            if (!get_type_info(reference.object_type, type_info, false, detail)) {
                return false;
            }
            auto* type = CastToObjectType(type_info);
            if (type == nullptr) {
                detail = "saved method reference resolves to a non-object type";
                return false;
            }
            if (type->GetNamespace() != nullptr && *type->GetNamespace() != '\0') {
                resolved_owner_name.assign(type->GetNamespace());
                resolved_owner_name.append("::");
            }
            if (type->GetName() != nullptr) resolved_owner_name.append(type->GetName());
            const auto match_method_candidate = [&](asCScriptFunction* candidate) {
                remember_candidate(candidate);
                if (!signature_matches(reference, *candidate, detail, type)) {
                    return false;
                }
                if (candidate->traits.GetTrait(asTRAIT_GENERIC_TEMPLATE_FUNCTION) &&
                    type->templateBaseType != nullptr) {
                    candidate = engine_.GenerateTemplateFunction(type, candidate);
                    if (candidate == nullptr) {
                        detail = "engine could not instantiate a saved generic template method";
                        return true;
                    }
                }
                found = candidate;
                return true;
            };
            type->FindMethodUntil(
                reference.name.bytes.c_str(),
                [&](asCScriptFunction* candidate) {
                    return match_method_candidate(candidate);
                });
            const auto check_behaviour = [&](const int id) {
                if (found != nullptr || id == 0) {
                    return;
                }
                asCScriptFunction* candidate = engine_.GetScriptFunction(id);
                if (candidate != nullptr && candidate->name == reference.name.bytes.c_str()) {
                    (void)match_method_candidate(candidate);
                }
            };
            check_behaviour(type->beh.factory);
            check_behaviour(type->beh.copyfactory);
            check_behaviour(type->beh.construct);
            check_behaviour(type->beh.copyconstruct);
            check_behaviour(type->beh.destruct);
            check_behaviour(type->beh.copy);
            check_behaviour(type->beh.templateCallback);
            for (asUINT index = 0U; index < type->beh.constructors.GetLength(); ++index) {
                check_behaviour(type->beh.constructors[index]);
            }
            for (asUINT index = 0U; index < type->beh.factories.GetLength(); ++index) {
                check_behaviour(type->beh.factories[index]);
            }
        } else {
            asSNameSpace* const ns = name_space(engine_, reference.name_space);
            if (!reference.module.bytes.empty()) {
                auto* module = static_cast<asCModule*>(
                    engine_.GetModule(reference.module.bytes.c_str(), false));
                if (module != nullptr) {
                    module->globalFunctions.FindAllUntil(
                        reference.name.bytes.c_str(), ns,
                        [&](asCScriptFunction* candidate) {
                            if (candidate != nullptr &&
                                signature_matches(reference, *candidate, detail)) {
                                found = candidate;
                                return true;
                            }
                            return false;
                        });
                }
            } else {
                engine_.registeredGlobalFuncTable.FindAllUntil(
                    reference.name.bytes.c_str(), ns,
                    [&](asCScriptFunction* candidate) {
                        if (signature_matches(reference, *candidate, detail)) {
                            found = candidate;
                            return true;
                        }
                        return false;
                    });
            }
        }
        if (found == nullptr) {
            if (detail.empty()) {
                detail = "saved function reference could not be resolved by its signature"
                    " (pointer=" + std::to_string(old_reference) +
                    ", name=" + reference.name.bytes +
                    ", module=" + reference.module.bytes +
                    ", namespace=" + reference.name_space.bytes +
                    ", is_const=" + std::to_string(reference.is_const) +
                    ", is_imported=" + std::to_string(reference.is_imported_decl) +
                    ", is_method=" + std::to_string(reference.is_method) +
                    ", object_type=" + std::to_string(reference.object_type) +
                    ", resolved_owner=" + resolved_owner_name +
                    ", parameters=" + std::to_string(reference.parameter_types.size()) +
                    ", candidates=" + candidate_declarations + ")";
            }
            return false;
        }
        function_cache_.emplace(old_reference, found);
        output = found;
        if (add_ref) {
            output->AddRefInternal();
        }
        if (mark_in_use) {
            output->isInUse = true;
        }
        return true;
    }

    bool get_global(
        const std::int64_t old_reference,
        void*& address,
        asCGlobalProperty*& property,
        std::string& detail) {
        if (old_reference == 0) {
            address = nullptr;
            property = nullptr;
            return true;
        }
        const auto cached = global_cache_.find(old_reference);
        if (cached != global_cache_.end()) {
            property = cached->second;
            address = property->memory;
            return true;
        }
        const auto saved = global_references_.find(old_reference);
        if (saved == global_references_.end()) {
            detail = "bytecode references an absent global";
            return false;
        }
        const global_reference& reference = *saved->second;
        if (reference.is_string) {
            if (engine_.stringFactory == nullptr) {
                detail = "string global requires a registered string factory";
                return false;
            }
            address = const_cast<void*>(engine_.stringFactory->GetStringConstant(
                reference.name.bytes.data(),
                static_cast<asUINT>(reference.name.bytes.size())));
            property = nullptr;
            if (address == nullptr) {
                detail = "registered string factory rejected a cached literal";
                return false;
            }
            return true;
        }
        asSNameSpace* const ns = name_space(engine_, reference.name_space);
        property = nullptr;
        if (!reference.module.bytes.empty()) {
            auto* module = static_cast<asCModule*>(
                engine_.GetModule(reference.module.bytes.c_str(), false));
            if (module != nullptr) {
                property = module->scriptGlobals.FindFirst(reference.name.bytes.c_str(), ns);
            }
        } else {
            property = engine_.registeredGlobalPropTable.FindFirst(
                reference.name.bytes.c_str(), ns);
        }
        if (property == nullptr || property->memory == nullptr) {
            detail = "saved global reference could not be resolved to allocated storage";
            return false;
        }
        global_cache_.emplace(old_reference, property);
        address = property->memory;
        return true;
    }

    bool get_property_offset(
        const int old_offset,
        const int old_type_id,
        short& output,
        int& declaring_type_id,
        std::string& detail) {
        if (old_offset < 0) {
            detail = "saved property offset is negative";
            return false;
        }
        const std::uint64_t key_bits =
            (static_cast<std::uint64_t>(static_cast<std::uint32_t>(old_type_id)) << 1U) |
            (static_cast<std::uint64_t>(static_cast<std::uint32_t>(old_offset)) << 33U) | 1U;
        const auto key = static_cast<std::int64_t>(key_bits);
        const auto cached = property_cache_.find(key);
        if (cached != property_cache_.end()) {
            output = cached->second.first;
            declaring_type_id = cached->second.second;
            return true;
        }
        const auto saved = property_references_.find(key);
        if (saved == property_references_.end()) {
            if (old_offset > std::numeric_limits<short>::max()) {
                detail = "unmapped property offset exceeds bytecode's signed-short operand";
                return false;
            }
            output = static_cast<short>(old_offset);
            declaring_type_id = -1;
            property_cache_.emplace(
                key, std::make_pair(output, declaring_type_id));
            return true;
        }
        int new_type_id = 0;
        if (!get_type_id(saved->second->old_type_id, new_type_id, false, detail)) {
            return false;
        }
        auto* type = static_cast<asCObjectType*>(engine_.GetTypeInfoById(new_type_id));
        asCObjectProperty* property = type == nullptr
            ? nullptr
            : type->GetFirstProperty(saved->second->name.bytes.c_str());
        if (property == nullptr || property->byteOffset < 0 ||
            property->byteOffset > std::numeric_limits<short>::max()) {
            detail = "saved property reference could not be resolved to a valid byte offset";
            return false;
        }
        output = static_cast<short>(property->byteOffset);
        declaring_type_id = new_type_id;
        property_cache_.emplace(
            key, std::make_pair(output, declaring_type_id));
        return true;
    }

    asCScriptEngine& engine_;
    std::unordered_map<std::int64_t, const type_reference*> type_references_;
    std::unordered_map<std::int32_t, std::int64_t> type_ids_;
    std::unordered_map<std::int64_t, const function_reference*> function_references_;
    std::unordered_map<std::int32_t, std::int64_t> function_ids_;
    std::unordered_map<std::int64_t, const global_reference*> global_references_;
    std::unordered_map<std::int64_t, const property_reference*> property_references_;
    std::unordered_map<std::int64_t, asCTypeInfo*> type_cache_;
    std::unordered_map<std::int64_t, asCScriptFunction*> function_cache_;
    std::unordered_map<std::int64_t, asCGlobalProperty*> global_cache_;
    std::unordered_map<std::int64_t, std::pair<short, int>> property_cache_;
    std::unordered_map<asCScriptFunction*, const std::vector<std::int32_t>*>
        pending_bytecode_;
};

bool create_function(
    asCScriptEngine& engine,
    asCModule& module,
    const precompiled_function& input,
    const int script_section_index,
    reference_resolver& references,
    asCScriptFunction*& output,
    std::string& detail) {
    asCScriptFunction* function = asNEW(asCScriptFunction)(&engine, &module, asFUNC_DUMMY);
    function->funcType = asFUNC_SCRIPT;
    function->name = input.function_name.bytes.c_str();
    function->nameSpace = name_space(engine, input.name_space);
    if (!references.create_data_type(input.return_type, function->returnType, false, detail)) {
        asDELETE(function, asCScriptFunction);
        return false;
    }

    const asUINT parameter_count = static_cast<asUINT>(input.parameter_types.size());
    function->parameterTypes.SetLength(parameter_count);
    function->parameterNames.SetLength(parameter_count);
    function->inOutFlags.SetLength(parameter_count);
    function->defaultArgs.SetLength(parameter_count);
    for (asUINT index = 0U; index < parameter_count; ++index) {
        if (!references.create_data_type(
                input.parameter_types[index], function->parameterTypes[index], false, detail)) {
            asDELETE(function, asCScriptFunction);
            return false;
        }
        function->parameterNames[index] = input.parameter_names[index].bytes.c_str();
        function->inOutFlags[index] =
            static_cast<asETypeModifiers>(input.parameter_flags[index]);
        function->defaultArgs[index] = nullptr;
        if (!input.parameter_default_args[index].bytes.empty()) {
            function->defaultArgs[index] =
                asNEW(asCString)(input.parameter_default_args[index].bytes.c_str());
        }
    }
    function->traits.traits = static_cast<asDWORD>(input.function_traits);
    function->AllocateScriptFunctionData();
    auto* const script = function->scriptData;
    references.stage_function_bytecode(*function, input.byte_code);
    script->variableSpace = input.variable_space;
    const asUINT object_variable_count =
        static_cast<asUINT>(input.object_variable_types.size());
    script->objVariableTypes.SetLength(object_variable_count);
    script->objVariablePos.SetLength(object_variable_count);
    for (asUINT index = 0U; index < object_variable_count; ++index) {
        asCTypeInfo* type = nullptr;
        if (!references.get_type_info(
                input.object_variable_types[index], type, false, detail) || type == nullptr) {
            asDELETE(function, asCScriptFunction);
            return false;
        }
        script->objVariableTypes[index] = type;
        script->objVariablePos[index] = input.object_variable_positions[index];
    }
    script->objVariablesOnHeap = input.object_variables_on_heap;
    script->stackNeeded = input.stack_needed;
    const asUINT variable_info_count =
        static_cast<asUINT>(input.variable_info_program_positions.size());
    script->objVariableInfo.SetLength(variable_info_count);
    for (asUINT index = 0U; index < variable_info_count; ++index) {
        script->objVariableInfo[index].programPos = input.variable_info_program_positions[index];
        script->objVariableInfo[index].variableOffset = input.variable_info_offsets[index];
        script->objVariableInfo[index].option =
            static_cast<asEObjVarInfoOption>(input.variable_info_options[index]);
    }
    script->declaredAt = input.declared_at;
    script->scriptSectionIdx = script_section_index;
    script->lineNumbers.SetLength(static_cast<asUINT>(input.line_numbers.size()));
    for (asUINT index = 0U; index < script->lineNumbers.GetLength(); ++index) {
        script->lineNumbers[index] = input.line_numbers[index];
    }
    output = function;
    return true;
}

asCObjectType* create_class_shell(
    asCScriptEngine& engine,
    asCModule& module,
    const precompiled_class& input) {
    asCObjectType* type = asNEW(asCObjectType)(&engine);
    type->typeId = engine.typeIdSeqNbr++;
    engine.mapTypeIdToTypeInfo.Add(type->typeId, type);
    type->name = input.class_name.bytes.c_str();
    type->nameSpace = name_space(engine, input.name_space);
    type->flags = static_cast<asDWORD>(input.flags);
    type->size = -1;
    type->module = &module;
    module.classTypes.PushLast(type);
    module.allLocalTypes.Add(type);
    engine.allScriptDeclaredTypes.Add(type);
    return type;
}

bool create_class_properties(
    asCObjectType& type,
    const precompiled_class& input,
    reference_resolver& references,
    std::string& detail) {
    if (type.derivedFrom != nullptr) {
        type.size = type.derivedFrom->size;
        type.alignment = type.derivedFrom->alignment;
    } else if (type.shadowType != nullptr) {
        type.size = type.basePropertyOffset;
        type.alignment = std::max(1, type.shadowType->alignment);
    } else {
        type.size = 0;
        type.alignment = 8;
    }
    type.properties.SetLength(0);
    type.properties.AllocateNoConstruct(static_cast<asUINT>(input.properties.size()), false);
    type.localProperties.AllocateNoConstruct(
        static_cast<asUINT>(input.properties.size()), false);
    type.propertyTable.Reserve(static_cast<asUINT>(input.properties.size()));
    for (const precompiled_property& input_property : input.properties) {
        auto* property = asNEW(asCObjectProperty)();
        property->name = input_property.name.bytes.c_str();
        if (!references.create_data_type(
                input_property.type, property->type, false, detail)) {
            asDELETE(property, asCObjectProperty);
            return false;
        }
        property->isPrivate = input_property.is_private;
        property->isProtected = input_property.is_protected;
        type.localProperties.PushLast(property);
        type.properties.PushLast(property);
        type.propertyTable.Add(property);
        if (asCTypeInfo* property_type = property->type.GetTypeInfo()) {
            property_type->AddRefInternal();
        }

        const asUINT alignment = property->type.GetAlignment();
        type.size = static_cast<int>(Align(static_cast<std::size_t>(type.size), alignment));
        type.alignment = std::max(type.alignment, static_cast<int>(alignment));
        property->byteOffset = type.size;
        if (property->type.IsObject()) {
            if ((property->type.GetTypeInfo()->flags & asOBJ_VALUE) != 0U) {
                type.size += property->type.GetSizeInMemoryBytes();
            } else {
                type.size += property->type.GetSizeOnStackDWords() * 4;
            }
        } else if (property->type.IsFuncdef()) {
            type.size += AS_PTR_SIZE * 4;
        } else {
            type.size += property->type.GetSizeInMemoryBytes();
        }
    }
    type.size = static_cast<int>(
        Align(static_cast<std::size_t>(type.size), static_cast<std::size_t>(type.alignment)));
    return true;
}

class class_layout_replayer final {
public:
    explicit class_layout_replayer(reference_resolver& references) : references_(references) {}

    void add(
        asCObjectType& type,
        const precompiled_class& record,
        const std::size_t module_index) {
        records_.emplace(&type, record_location{&record, module_index});
        order_.push_back(&type);
    }

    bool process_all(std::size_t& module_index, std::string& detail) {
        for (asCObjectType* const type : order_) {
            if (!process(*type, module_index, detail)) {
                return false;
            }
        }
        return true;
    }

private:
    struct record_location {
        const precompiled_class* record = nullptr;
        std::size_t module_index = kNoModule;
    };

    bool process(asCObjectType& type, std::size_t& module_index, std::string& detail) {
        const auto current = states_.find(&type);
        if (current != states_.end()) {
            if (current->second == 2U) {
                return true;
            }
            module_index = records_.at(&type).module_index;
            detail = "cycle detected while replaying script class layouts";
            return false;
        }
        states_.emplace(&type, 1U);
        const record_location location = records_.at(&type);
        module_index = location.module_index;
        const precompiled_class& record = *location.record;

        type.derivedFrom = nullptr;
        if (record.derived_from != 0) {
            asCTypeInfo* base_info = nullptr;
            if (!references_.get_type_info(
                    record.derived_from, base_info, false, detail) || base_info == nullptr) {
                return false;
            }
            asCObjectType* base = CastToObjectType(base_info);
            if (base == nullptr || (base->flags & asOBJ_SCRIPT_OBJECT) == 0U) {
                detail = "saved script base reference does not resolve to a script object type";
                return false;
            }
            const auto saved_base = records_.find(base);
            if (saved_base != records_.end() && !process(*base, module_index, detail)) {
                return false;
            }
            type.derivedFrom = base;
        }

        for (const precompiled_property& property : record.properties) {
            if (property.type.type_info == 0) {
                continue;
            }
            asCTypeInfo* property_type = nullptr;
            if (!references_.get_type_info(
                    property.type.type_info, property_type, false, detail) ||
                property_type == nullptr) {
                return false;
            }
            asCObjectType* object_type = CastToObjectType(property_type);
            if (object_type != nullptr && (object_type->flags & asOBJ_VALUE) != 0U) {
                const auto saved_property_type = records_.find(object_type);
                if (saved_property_type != records_.end() &&
                    !process(*object_type, module_index, detail)) {
                    return false;
                }
            }
        }

        if (!create_class_properties(type, record, references_, detail)) {
            return false;
        }
        states_[&type] = 2U;
        module_index = location.module_index;
        return true;
    }

    reference_resolver& references_;
    std::unordered_map<asCObjectType*, record_location> records_;
    std::unordered_map<asCObjectType*, std::uint8_t> states_;
    std::vector<asCObjectType*> order_;
};

bool attach_class_function(
    asCScriptEngine& engine,
    asCModule& module,
    asCObjectType& type,
    const precompiled_function& input,
    const int script_section_index,
    reference_resolver& references,
    asCScriptFunction*& output,
    std::string& detail) {
    asCScriptFunction* function = nullptr;
    if (!create_function(
            engine, module, input, script_section_index,
            references, function, detail)) {
        return false;
    }
    function->objectType = &type;
    type.AddRefInternal();
    function->id = engine.GetNextScriptFunctionId();
    function->CalculateParameterOffsets();
    module.AddScriptFunction(function);
    output = function;
    return true;
}

bool create_class_functions(
    asCScriptEngine& engine,
    asCModule& module,
    asCObjectType& type,
    const precompiled_class& input,
    const int script_section_index,
    reference_resolver& references,
    std::string& detail) {
    const bool is_value_type = (type.flags & asOBJ_VALUE) != 0U;
    if (!is_value_type) {
        type.virtualFunctionTable.SetLength(static_cast<asUINT>(input.method_table.size()));
        type.methodTable.Reserve(static_cast<asUINT>(input.method_table.size()));
        for (asUINT slot = 0U; slot < type.virtualFunctionTable.GetLength(); ++slot) {
            const std::int32_t method_index = input.method_table[slot];
            if (method_index == -1) {
                type.virtualFunctionTable[slot] = nullptr;
                continue;
            }
            asCScriptFunction* function = nullptr;
            if (!attach_class_function(
                    engine, module, type, input.methods[method_index],
                    script_section_index, references, function, detail)) {
                return false;
            }
            function->vfTableIdx = static_cast<int>(slot);
            type.virtualFunctionTable[slot] = function;
            function->AddRefInternal();
            type.methods.PushLast(function->id);
            function->AddRefInternal();
            type.methodTable.Add(function);
        }
    } else {
        type.methods.SetLength(static_cast<asUINT>(input.methods.size()));
        type.methodTable.Reserve(static_cast<asUINT>(input.methods.size()));
        for (asUINT index = 0U; index < type.methods.GetLength(); ++index) {
            asCScriptFunction* function = nullptr;
            if (!attach_class_function(
                    engine, module, type, input.methods[index],
                    script_section_index, references, function, detail)) {
                return false;
            }
            function->AddRefInternal();
            type.methods[index] = function->id;
            type.methodTable.Add(function);
        }
    }

    for (const precompiled_function& constructor : input.constructors) {
        asCScriptFunction* function = nullptr;
        if (!attach_class_function(
                engine, module, type, constructor, script_section_index,
                references, function, detail)) {
            return false;
        }
        function->AddRefInternal();
        type.beh.constructors.PushLast(function->id);
        function->isInUse = true;
    }

    for (std::size_t index = 0U; index < input.behaviour_functions.size(); ++index) {
        asCScriptFunction* function = nullptr;
        if (!attach_class_function(
                engine, module, type, input.behaviour_functions[index],
                script_section_index, references, function, detail)) {
            return false;
        }
        function->AddRefInternal();
        function->isInUse = true;
        if (input.behaviour_function_types[index] == asBEHAVE_DESTRUCT) {
            type.beh.destruct = function->id;
        }
    }
    return true;
}

bool bind_class_function_references(
    asCObjectType& type,
    const precompiled_class& input,
    reference_resolver& references,
    std::string& detail) {
    for (const std::int64_t old_id : input.factory_references) {
        int id = 0;
        if (!references.get_function_id(old_id, id, true, detail)) {
            return false;
        }
        type.beh.factories.PushLast(id);
    }
    if (input.behaviour_references.empty()) {
        return true;
    }
    int resolved[7]{};
    for (std::size_t index = 0U; index < input.behaviour_references.size(); ++index) {
        // ReleaseAllFunctions owns/releases listFactory and copy directly.
        // Destructors already acquire that ownership when their serialized
        // function body is attached; factory/constructor slots are owned by
        // their parallel arrays instead of the scalar aliases.
        const bool add_direct_owner = index == 1U || index == 6U;
        if (!references.get_function_id(
                input.behaviour_references[index], resolved[index],
                add_direct_owner, detail)) {
            return false;
        }
    }
    type.beh.factory = resolved[0];
    type.beh.listFactory = resolved[1];
    type.beh.copyfactory = resolved[2];
    type.beh.construct = resolved[3];
    type.beh.copyconstruct = resolved[4];
    type.beh.destruct = resolved[5];
    type.beh.copy = resolved[6];
    return true;
}

class class_function_preprocessor final {
public:
    void add(
        asCObjectType& type,
        const precompiled_class& record,
        const std::size_t module_index) {
        records_.emplace(&type, record_location{&record, module_index});
        order_.push_back(&type);
    }

    bool process_all(std::size_t& module_index, std::string& detail) {
        for (asCObjectType* const type : order_) {
            if (!process(*type, module_index, detail)) {
                return false;
            }
        }
        return true;
    }

private:
    struct record_location {
        const precompiled_class* record = nullptr;
        std::size_t module_index = kNoModule;
    };

    bool process(asCObjectType& type, std::size_t& module_index, std::string& detail) {
        const auto state = states_.find(&type);
        if (state != states_.end()) {
            return state->second == 2U;
        }
        states_.emplace(&type, 1U);
        const record_location location = records_.at(&type);
        module_index = location.module_index;
        if (type.derivedFrom != nullptr) {
            const auto base_record = records_.find(type.derivedFrom);
            if (base_record != records_.end() &&
                !process(*type.derivedFrom, module_index, detail)) {
                return false;
            }
            for (asUINT index = 0U; index < type.derivedFrom->properties.GetLength(); ++index) {
                asCObjectProperty* property = type.derivedFrom->properties[index];
                if (property->byteOffset < type.basePropertyOffset) {
                    continue;
                }
                type.properties.PushLast(property);
                type.propertyTable.Add(property);
            }
        }

        if ((type.flags & asOBJ_VALUE) == 0U) {
            for (asUINT slot = 0U; slot < type.virtualFunctionTable.GetLength(); ++slot) {
                if (type.virtualFunctionTable[slot] != nullptr) {
                    continue;
                }
                asCScriptFunction* inherited = nullptr;
                for (asCObjectType* base = type.derivedFrom;
                     base != nullptr; base = base->derivedFrom) {
                    if (slot < base->virtualFunctionTable.GetLength() &&
                        base->virtualFunctionTable[slot] != nullptr) {
                        inherited = base->virtualFunctionTable[slot];
                        break;
                    }
                }
                if (inherited == nullptr) {
                    detail = "class virtual slot has no implementation in its base chain";
                    return false;
                }
                type.virtualFunctionTable[slot] = inherited;
                inherited->AddRefInternal();
                type.methods.PushLast(inherited->id);
                inherited->AddRefInternal();
                type.methodTable.Add(inherited);
            }
        }
        states_[&type] = 2U;
        module_index = location.module_index;
        return true;
    }

    std::unordered_map<asCObjectType*, record_location> records_;
    std::unordered_map<asCObjectType*, std::uint8_t> states_;
    std::vector<asCObjectType*> order_;
};

bool add_function_import(
    asCScriptEngine& engine,
    asCModule& module,
    const function_import& input,
    reference_resolver& references,
    std::string& detail) {
    const function_signature& source = input.signature;
    asCDataType return_type;
    if (!references.create_data_type(source.return_type, return_type, false, detail)) {
        return false;
    }
    const asUINT count = static_cast<asUINT>(source.parameter_types.size());
    asCArray<asCDataType> parameters;
    asCArray<asETypeModifiers> flags;
    asCArray<asCString*> defaults;
    parameters.SetLength(count);
    flags.SetLength(count);
    defaults.SetLength(count);
    for (asUINT index = 0U; index < count; ++index) {
        defaults[index] = nullptr;
    }
    for (asUINT index = 0U; index < count; ++index) {
        if (!references.create_data_type(
                source.parameter_types[index], parameters[index], false, detail)) {
            for (asUINT cleanup = 0U; cleanup < count; ++cleanup) {
                if (defaults[cleanup] != nullptr) {
                    asDELETE(defaults[cleanup], asCString);
                }
            }
            return false;
        }
        flags[index] = static_cast<asETypeModifiers>(source.parameter_flags[index]);
        if (!source.parameter_default_args[index].bytes.empty()) {
            defaults[index] =
                asNEW(asCString)(source.parameter_default_args[index].bytes.c_str());
        }
    }
    asCString module_name = input.imported_from_module.bytes.c_str();
    const int result = module.AddImportedFunction(
        module.GetNextImportedFunctionId(), source.name.bytes.c_str(), return_type,
        parameters, flags, defaults, name_space(engine, source.name_space), module_name);
    if (result < 0) {
        detail = "engine rejected a saved imported-function signature";
        return false;
    }
    return true;
}

void create_enum(
    asCScriptEngine& engine,
    asCModule& module,
    const precompiled_enum& input) {
    asCEnumType* type = asNEW(asCEnumType)(&engine);
    type->name = input.name.bytes.c_str();
    type->flags = asOBJ_ENUM;
    type->size = 1;
    type->alignment = 1;
    type->module = &module;
    type->nameSpace = name_space(engine, input.name_space);
    module.enumTypes.PushLast(type);
    module.allLocalTypes.Add(type);
    type->enumValues.SetLength(static_cast<asUINT>(input.names.size()));
    for (asUINT index = 0U; index < type->enumValues.GetLength(); ++index) {
        type->enumValues[index] = asNEW(asSEnumValue)();
        type->enumValues[index]->name = input.names[index].bytes.c_str();
        type->enumValues[index]->value = input.values[index];
    }
}

bool create_global(
    asCScriptEngine& engine,
    asCModule& module,
    const precompiled_global& input,
    const int script_section_index,
    reference_resolver& references,
    asCGlobalProperty*& output,
    std::string& detail) {
    asCDataType type;
    if (!references.create_data_type(input.type, type, false, detail)) {
        return false;
    }
    asCGlobalProperty* property =
        module.AllocateGlobalProperty(input.name.bytes.c_str(), type, name_space(engine, input.name_space));
    if (input.is_pure_constant) {
        property->isPureConstant = true;
        property->storage = input.pure_constant_value;
    } else if (input.is_default_init) {
        property->isDefaultInit = true;
    } else if (input.has_init_function) {
        asCScriptFunction* function = nullptr;
        if (!create_function(
                engine, module, input.init_function, script_section_index,
                references, function, detail)) {
            return false;
        }
        function->id = engine.GetNextScriptFunctionId();
        function->CalculateParameterOffsets();
        engine.AddScriptFunction(function);
        property->SetInitFunc(function);
    }
    output = property;
    return true;
}

class build_guard final {
public:
    explicit build_guard(asCScriptEngine& engine) noexcept : engine_(engine) {}
    ~build_guard() {
        if (requested_) {
            engine_.BuildCompleted();
        }
    }
    int request() {
        const int result = engine_.RequestBuild();
        requested_ = result >= 0;
        return result;
    }

private:
    asCScriptEngine& engine_;
    bool requested_ = false;
};

class module_cleanup final {
public:
    explicit module_cleanup(asCScriptEngine& engine) noexcept : engine_(engine) {}
    ~module_cleanup() {
        if (keep_) {
            return;
        }
        for (auto name = names_.rbegin(); name != names_.rend(); ++name) {
            engine_.DiscardModule((*name)->c_str());
        }
    }

    void reserve(const std::size_t count) { names_.reserve(count); }
    void add(const std::string& name) noexcept { names_.push_back(&name); }
    void keep() noexcept { keep_ = true; }

private:
    asCScriptEngine& engine_;
    std::vector<const std::string*> names_;
    bool keep_ = false;
};

class reference_exporter final {
public:
    reference_exporter(asCScriptEngine& engine, cache& output) : engine_(engine), output_(output) {
        for (const auto& entry : output_.type_references) {
            type_keys_.emplace(entry.first);
        }
        for (const auto& entry : output_.type_id_reference_to_pointer) {
            type_id_keys_.emplace(entry.first);
        }
        for (const auto& entry : output_.function_references) {
            function_keys_.emplace(entry.first);
        }
        for (const auto& entry : output_.function_id_reference_to_pointer) {
            function_id_keys_.emplace(entry.first);
        }
        for (const auto& entry : output_.global_references) {
            global_keys_.emplace(entry.first);
        }
        for (const auto& entry : output_.property_references) {
            property_keys_.emplace(entry.first);
        }
    }

    bool export_data_type(
        const asCDataType& input,
        data_type& output,
        std::string& detail) {
        output.is_reference = input.IsReference();
        output.is_object_const = input.IsObjectConst();
        output.is_object_handle = input.IsObjectHandle();
        output.is_const_handle = input.IsReadOnly();
        output.is_auto = input.IsAuto();
        output.if_handle_then_const = input.HasIfHandleThenConst();
        output.token_type = static_cast<std::int32_t>(input.GetTokenType());
        return reference_type_info(input.GetTypeInfo(), output.type_info, detail);
    }

    bool store_bytecode(std::vector<std::int32_t>& bytecode, std::string& detail) {
        asDWORD* instruction = reinterpret_cast<asDWORD*>(bytecode.data());
        asDWORD* const end = instruction + bytecode.size();
        while (instruction < end) {
            const auto opcode = static_cast<asEBCInstr>(*reinterpret_cast<asBYTE*>(instruction));
            switch (opcode) {
            case asBC_PshGPtr:
            case asBC_PshG4:
            case asBC_LdGRdR4:
            case asBC_CpyVtoG4:
            case asBC_CpyGtoV4:
            case asBC_LDG:
            case asBC_PGA:
            case asBC_SetG4:
                if (!reference_global(
                        reinterpret_cast<void*>(asBC_PTRARG(instruction)), detail)) {
                    return false;
                }
                break;
            case asBC_CALL:
            case asBC_CALLBND:
            case asBC_CALLINTF:
                if (!reference_function_id(asBC_INTARG(instruction), detail)) {
                    return false;
                }
                break;
            case asBC_CopyScript:
            case asBC_FinConstruct:
            case asBC_FREE:
            case asBC_DestructScript:
            case asBC_OBJTYPE: {
                std::int64_t ignored = 0;
                if (!reference_type_info(
                        reinterpret_cast<asCTypeInfo*>(asBC_PTRARG(instruction)),
                        ignored, detail)) {
                    return false;
                }
                break;
            }
            case asBC_COPY:
            case asBC_TYPEID:
            case asBC_Cast:
                if (!reference_type_id(asBC_INTARG(instruction), detail)) {
                    return false;
                }
                break;
            case asBC_STR:
                detail = "fork compiler emitted unsupported asBC_STR while exporting";
                return false;
            case asBC_CALLSYS:
            case asBC_FuncPtr:
            case asBC_Thiscall1:
                if (!reference_function(
                        reinterpret_cast<asCScriptFunction*>(asBC_PTRARG(instruction)), detail)) {
                    return false;
                }
                break;
            case asBC_ALLOC: {
                std::int64_t ignored = 0;
                if (!reference_type_info(
                        reinterpret_cast<asCTypeInfo*>(asBC_PTRARG(instruction)),
                        ignored, detail) ||
                    !reference_function_id(asBC_INTARG(instruction + AS_PTR_SIZE), detail)) {
                    return false;
                }
                break;
            }
            case asBC_SetListType:
                if (!reference_type_id(asBC_INTARG(instruction + 1), detail)) {
                    return false;
                }
                break;
            case asBC_ADDSi:
            case asBC_LoadThisR: {
                int declaring_type_id = 0;
                if (!reference_property(
                        asBC_SWORDARG0(instruction), asBC_INTARG(instruction),
                        declaring_type_id, detail)) {
                    return false;
                }
                asBC_INTARG(instruction) = declaring_type_id;
                break;
            }
            case asBC_LoadRObjR:
            case asBC_LoadVObjR: {
                auto* const type_id = reinterpret_cast<int*>(instruction + 2);
                int declaring_type_id = 0;
                if (!reference_property(
                        asBC_SWORDARG1(instruction), *type_id,
                        declaring_type_id, detail)) {
                    return false;
                }
                *type_id = declaring_type_id;
                break;
            }
            default: break;
            }
            const int size = asBCTypeSize[asBCInfo[opcode].type];
            if (size <= 0 || size > end - instruction) {
                detail = "exporter lost the validated bytecode instruction boundary";
                return false;
            }
            instruction += size;
        }
        return instruction == end;
    }

    bool reference_type_info(
        asCTypeInfo* type,
        std::int64_t& key,
        std::string& detail) {
        if (type == nullptr) {
            key = 0;
            return true;
        }
        key = static_cast<std::int64_t>(reinterpret_cast<std::intptr_t>(type));
        if (type_keys_.find(key) != type_keys_.end()) {
            return true;
        }
        type_keys_.emplace(key);
        output_.type_references.emplace_back(key, type_reference{});
        const std::size_t output_index = output_.type_references.size() - 1U;
        type_reference encoded;
        encoded.name.bytes = type->GetName();
        if ((type->GetFlags() & asOBJ_TEMPLATE_SUBTYPE) == 0U) {
            encoded.name_space.bytes = type->GetNamespace();
            if (asIScriptModule* module = type->GetModule()) {
                encoded.module.bytes = module->GetName();
            }
            const asUINT subtype_count = type->GetSubTypeCount();
            auto* object_type = CastToObjectType(type);
            if (subtype_count != 0U && object_type != nullptr &&
                object_type->templateBaseType != nullptr) {
                encoded.sub_types.resize(subtype_count);
                for (asUINT index = 0U; index < subtype_count; ++index) {
                    if (!export_data_type(
                            object_type->templateSubTypes[index],
                            encoded.sub_types[index], detail)) {
                        return false;
                    }
                }
            }
        } else {
            encoded.module.bytes = "$__T__";
            bool found_owner = false;
            for (asUINT index = 0U;
                 !found_owner && index < engine_.registeredTemplateTypes.GetLength(); ++index) {
                asCObjectType* template_type = engine_.registeredTemplateTypes[index];
                for (asUINT subtype = 0U;
                     subtype < template_type->templateSubTypes.GetLength(); ++subtype) {
                    if (template_type->templateSubTypes[subtype].GetTypeInfo() == type) {
                        encoded.name_space.bytes = template_type->GetName();
                        found_owner = true;
                        break;
                    }
                }
            }
            if (!found_owner) {
                detail = "template subtype has no registered template owner";
                return false;
            }
        }
        output_.type_references[output_index].second = std::move(encoded);
        const std::int32_t type_id = type->GetTypeId();
        if (type_id > asTYPEID_LAST_PRIMITIVE && type_id_keys_.emplace(type_id).second) {
            output_.type_id_reference_to_pointer.emplace_back(type_id, key);
        }
        return true;
    }

    bool store_function_id(const int id, std::string& detail) {
        return reference_function_id(id, detail);
    }

private:
    bool reference_type_id(const int id, std::string& detail) {
        if (id == 0 || id <= asTYPEID_LAST_PRIMITIVE) {
            return true;
        }
        asCTypeInfo* type = static_cast<asCTypeInfo*>(engine_.GetTypeInfoById(id));
        if (type == nullptr) {
            detail = "bytecode type id does not resolve in the source engine";
            return false;
        }
        std::int64_t key = 0;
        return reference_type_info(type, key, detail);
    }

    bool reference_function(asCScriptFunction* function, std::string& detail) {
        if (function == nullptr) {
            return true;
        }
        const auto key = static_cast<std::int64_t>(reinterpret_cast<std::intptr_t>(function));
        if (function_keys_.find(key) != function_keys_.end()) {
            return true;
        }
        function_keys_.emplace(key);
        function_reference encoded;
        encoded.name.bytes = function->name.AddressOf();
        encoded.name_space.bytes = function->GetNamespace();
        if (function->module != nullptr) {
            encoded.module.bytes = function->module->GetName();
        }
        encoded.parameter_types.resize(function->parameterTypes.GetLength());
        for (asUINT index = 0U; index < function->parameterTypes.GetLength(); ++index) {
            if (!export_data_type(
                    function->parameterTypes[index], encoded.parameter_types[index], detail)) {
                return false;
            }
        }
        if (function->objectType != nullptr) {
            encoded.is_const = function->IsReadOnly();
            encoded.is_method = true;
            if (!reference_type_info(function->objectType, encoded.object_type, detail)) {
                return false;
            }
        } else if (function->name == "$beh3" || function->name == "$fact") {
            encoded.is_method = true;
            if (!reference_type_info(
                    function->returnType.GetTypeInfo(), encoded.object_type, detail)) {
                return false;
            }
        }
        if (!export_data_type(function->returnType, encoded.return_type, detail)) {
            return false;
        }
        encoded.is_imported_decl = function->GetFuncType() == asFUNC_IMPORTED;
        output_.function_references.emplace_back(key, std::move(encoded));
        const std::int32_t id = function->GetId();
        if (id != 0 && function_id_keys_.emplace(id).second) {
            output_.function_id_reference_to_pointer.emplace_back(id, key);
        }
        return true;
    }

    bool reference_function_id(const int id, std::string& detail) {
        if (id == 0) {
            return true;
        }
        constexpr int kImportedFunction = 0x40000000;
        asCScriptFunction* function = nullptr;
        if ((id & kImportedFunction) != 0) {
            const int index = id & ~kImportedFunction;
            if (index >= 0 && static_cast<asUINT>(index) < engine_.importedFunctions.GetLength() &&
                engine_.importedFunctions[index] != nullptr) {
                function = engine_.importedFunctions[index]->importedFunctionSignature;
            }
        } else {
            function = engine_.GetScriptFunction(id);
        }
        if (function == nullptr || function->GetId() != id) {
            detail = "bytecode function id does not resolve in the source engine";
            return false;
        }
        return reference_function(function, detail);
    }

    bool reference_global(void* address, std::string& detail) {
        if (address == nullptr) {
            return true;
        }
        const auto key = static_cast<std::int64_t>(reinterpret_cast<std::intptr_t>(address));
        if (global_keys_.find(key) != global_keys_.end()) {
            return true;
        }
        asCGlobalProperty** property_pointer = engine_.varAddressMap.Find(address);
        global_reference encoded;
        if (property_pointer == nullptr || *property_pointer == nullptr) {
            if (engine_.stringFactory == nullptr) {
                detail = "global address is neither a property nor a string constant";
                return false;
            }
            asUINT length = 0U;
            if (engine_.stringFactory->GetRawStringData(address, nullptr, &length) < 0) {
                detail = "string factory did not recognize a bytecode literal";
                return false;
            }
            encoded.name.bytes.resize(length);
            if (length != 0U && engine_.stringFactory->GetRawStringData(
                    address, encoded.name.bytes.data(), &length) < 0) {
                detail = "string factory failed to export a bytecode literal";
                return false;
            }
            encoded.is_string = true;
        } else {
            asCGlobalProperty* const property = *property_pointer;
            encoded.name.bytes = property->name.AddressOf();
            encoded.name_space.bytes = property->nameSpace->GetName();
            if (property->module != nullptr) {
                encoded.module.bytes = property->module->GetName();
            }
        }
        global_keys_.emplace(key);
        output_.global_references.emplace_back(key, std::move(encoded));
        return true;
    }

    bool reference_property(
        const int offset,
        const int type_id,
        int& declaring_type_id,
        std::string& detail) {
        auto* type = static_cast<asCObjectType*>(engine_.GetTypeInfoById(type_id));
        asCObjectType* declaring_type = nullptr;
        asCObjectProperty* property = type == nullptr
            ? nullptr
            : type->GetPropertyByOffset(offset, &declaring_type);
        if (property == nullptr || declaring_type == nullptr) {
            detail = "bytecode property operand does not resolve in the source engine";
            return false;
        }
        declaring_type_id = declaring_type->GetTypeId();
        const std::uint64_t bits =
            (static_cast<std::uint64_t>(static_cast<std::uint32_t>(declaring_type_id)) << 1U) |
            (static_cast<std::uint64_t>(static_cast<std::uint32_t>(offset)) << 33U) | 1U;
        const auto key = static_cast<std::int64_t>(bits);
        if (property_keys_.emplace(key).second) {
            if (!reference_type_id(declaring_type_id, detail)) {
                return false;
            }
            property_reference encoded;
            encoded.name.bytes = property->name.AddressOf();
            encoded.old_type_id = declaring_type_id;
            output_.property_references.emplace_back(key, std::move(encoded));
        }
        return true;
    }

    asCScriptEngine& engine_;
    cache& output_;
    std::unordered_set<std::int64_t> type_keys_;
    std::unordered_set<std::int32_t> type_id_keys_;
    std::unordered_set<std::int64_t> function_keys_;
    std::unordered_set<std::int32_t> function_id_keys_;
    std::unordered_set<std::int64_t> global_keys_;
    std::unordered_set<std::int64_t> property_keys_;
};

engine_bridge_result export_function(
    const asCScriptFunction& function,
    reference_exporter* references,
    precompiled_function& output) {
    if (function.scriptData == nullptr) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "checkpoint exporter requires script bytecode for every serialized function");
    }
    precompiled_function encoded;
    encoded.function_name.bytes = function.name.AddressOf();
    encoded.name_space.bytes = function.GetNamespace();
    std::string detail;
    if (references == nullptr ||
        !references->export_data_type(function.returnType, encoded.return_type, detail)) {
        if (references == nullptr && function.returnType.GetTypeInfo() == nullptr) {
            encoded.return_type.is_reference = function.returnType.IsReference();
            encoded.return_type.is_object_const = function.returnType.IsObjectConst();
            encoded.return_type.is_const_handle = function.returnType.IsReadOnly();
            encoded.return_type.token_type =
                static_cast<std::int32_t>(function.returnType.GetTokenType());
        } else {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                detail.empty() ? "function return type requires a cache reference exporter"
                               : std::move(detail));
        }
    }
    if (!valid_data_type_shape(encoded.return_type)) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "function return type could not be represented in the cache");
    }
    const asUINT parameter_count = function.parameterTypes.GetLength();
    encoded.parameter_types.resize(parameter_count);
    encoded.parameter_names.resize(parameter_count);
    encoded.parameter_flags.resize(parameter_count);
    encoded.parameter_default_args.resize(parameter_count);
    for (asUINT index = 0U; index < parameter_count; ++index) {
        if (references == nullptr) {
            const asCDataType& parameter = function.parameterTypes[index];
            if (parameter.GetTypeInfo() != nullptr) {
                return failure(
                    engine_bridge_phase::export_module, kNoModule,
                    "object parameter requires a cache reference exporter");
            }
            encoded.parameter_types[index].is_reference = parameter.IsReference();
            encoded.parameter_types[index].is_object_const = parameter.IsObjectConst();
            encoded.parameter_types[index].is_const_handle = parameter.IsReadOnly();
            encoded.parameter_types[index].token_type =
                static_cast<std::int32_t>(parameter.GetTokenType());
        } else if (!references->export_data_type(
                       function.parameterTypes[index], encoded.parameter_types[index], detail)) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                std::move(detail));
        }
        encoded.parameter_names[index].bytes = function.parameterNames[index].AddressOf();
        encoded.parameter_flags[index] = static_cast<std::int32_t>(function.inOutFlags[index]);
        if (function.defaultArgs[index] != nullptr) {
            encoded.parameter_default_args[index].bytes =
                function.defaultArgs[index]->AddressOf();
        }
    }
    encoded.function_traits = static_cast<std::int32_t>(function.traits.traits);
    const auto& script = *function.scriptData;
    encoded.byte_code.resize(script.byteCode.GetLength());
    if (!encoded.byte_code.empty()) {
        std::memcpy(
            encoded.byte_code.data(), script.byteCode.AddressOf(),
            encoded.byte_code.size() * sizeof(encoded.byte_code[0]));
    }
    encoded.variable_space = script.variableSpace;
    if (script.objVariableTypes.GetLength() != script.objVariablePos.GetLength()) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "compiled object-local arrays do not have identical lengths");
    }
    if (script.objVariableTypes.GetLength() != 0U && references == nullptr) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "object locals require a cache reference exporter");
    }
    encoded.object_variable_types.resize(script.objVariableTypes.GetLength());
    encoded.object_variable_positions.resize(script.objVariablePos.GetLength());
    for (asUINT index = 0U; index < script.objVariableTypes.GetLength(); ++index) {
        if (!references->reference_type_info(
                script.objVariableTypes[index], encoded.object_variable_types[index], detail)) {
            return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
        }
        encoded.object_variable_positions[index] = script.objVariablePos[index];
    }
    encoded.object_variables_on_heap = script.objVariablesOnHeap;
    encoded.variable_info_program_positions.resize(script.objVariableInfo.GetLength());
    encoded.variable_info_offsets.resize(script.objVariableInfo.GetLength());
    encoded.variable_info_options.resize(script.objVariableInfo.GetLength());
    for (asUINT index = 0U; index < script.objVariableInfo.GetLength(); ++index) {
        encoded.variable_info_program_positions[index] = script.objVariableInfo[index].programPos;
        encoded.variable_info_offsets[index] = script.objVariableInfo[index].variableOffset;
        encoded.variable_info_options[index] =
            static_cast<std::int32_t>(script.objVariableInfo[index].option);
    }
    encoded.stack_needed = script.stackNeeded;
    encoded.id = static_cast<std::uint32_t>(function.id);
    encoded.is_const_method = function.IsReadOnly();
    encoded.is_no_op = function.IsNoOp();
    // BuildID 24539464 is a Shipping build. The donor serializer compiles these fields out with
    // `#if !UE_BUILD_SHIPPING`, even though the AngelScript builder keeps them in memory.
    encoded.declared_at = 0;
    encoded.line_numbers.clear();
    if (references != nullptr) {
        if (!references->store_bytecode(encoded.byte_code, detail)) {
            return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
        }
    } else {
        std::size_t offset = 0U;
        while (offset < encoded.byte_code.size()) {
            const auto opcode = static_cast<asEBCInstr>(encoded.byte_code[offset] & 0xff);
            switch (opcode) {
            case asBC_PshGPtr: case asBC_PshG4: case asBC_LdGRdR4:
            case asBC_CpyVtoG4: case asBC_CpyGtoV4: case asBC_LDG:
            case asBC_PGA: case asBC_SetG4: case asBC_CALL: case asBC_CALLBND:
            case asBC_CALLINTF: case asBC_CopyScript: case asBC_FinConstruct:
            case asBC_FREE: case asBC_DestructScript: case asBC_OBJTYPE:
            case asBC_COPY: case asBC_STR: case asBC_CALLSYS: case asBC_FuncPtr:
            case asBC_Thiscall1: case asBC_ALLOC: case asBC_TYPEID: case asBC_Cast:
            case asBC_ADDSi: case asBC_LoadThisR: case asBC_LoadRObjR:
            case asBC_LoadVObjR: case asBC_SetListType:
                return failure(
                    engine_bridge_phase::export_module, kNoModule,
                    "reference-bearing bytecode requires a cache reference exporter");
            default: break;
            }
            offset += static_cast<std::size_t>(asBCTypeSize[asBCInfo[opcode].type]);
        }
    }
    if (!validate_function_shape(encoded, detail)) {
        return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
    }
    output = std::move(encoded);
    return {};
}

engine_bridge_result export_class(
    asCScriptEngine& engine,
    const asCObjectType& type,
    reference_exporter& references,
    precompiled_class& output) {
    precompiled_class encoded;
    encoded.class_name.bytes = type.GetName();
    encoded.name_space.bytes = type.GetNamespace();
    encoded.flags = static_cast<std::int32_t>(type.flags);

    const int first_local_offset = type.derivedFrom != nullptr
        ? type.derivedFrom->size
        : type.shadowType != nullptr ? type.basePropertyOffset : 0;
    for (asUINT index = 0U; index < type.properties.GetLength(); ++index) {
        const asCObjectProperty& property = *type.properties[index];
        if (property.byteOffset < first_local_offset) {
            continue;
        }
        precompiled_property property_record;
        property_record.name.bytes = property.name.AddressOf();
        std::string detail;
        if (!references.export_data_type(property.type, property_record.type, detail)) {
            return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
        }
        property_record.is_private = property.isPrivate;
        property_record.is_protected = property.isProtected;
        encoded.properties.push_back(std::move(property_record));
    }

    if ((type.flags & asOBJ_VALUE) == 0U) {
        encoded.method_table.reserve(type.virtualFunctionTable.GetLength());
        for (asUINT slot = 0U; slot < type.virtualFunctionTable.GetLength(); ++slot) {
            asCScriptFunction* function = type.virtualFunctionTable[slot];
            if (function == nullptr || function->objectType != &type) {
                encoded.method_table.push_back(-1);
                continue;
            }
            encoded.method_table.push_back(static_cast<std::int32_t>(encoded.methods.size()));
            precompiled_function method;
            engine_bridge_result result = export_function(*function, &references, method);
            if (!result.succeeded()) {
                return result;
            }
            encoded.methods.push_back(std::move(method));
        }
    } else {
        encoded.methods.reserve(type.methods.GetLength());
        for (asUINT index = 0U; index < type.methods.GetLength(); ++index) {
            asCScriptFunction* function = engine.GetScriptFunction(type.methods[index]);
            if (function == nullptr || function->objectType != &type) {
                return failure(
                    engine_bridge_phase::export_module, kNoModule,
                    "value-type method table contains a missing or foreign function", asERROR);
            }
            precompiled_function method;
            engine_bridge_result result = export_function(*function, &references, method);
            if (!result.succeeded()) {
                return result;
            }
            encoded.methods.push_back(std::move(method));
        }
    }

    std::string detail;
    if (!references.reference_type_info(type.derivedFrom, encoded.derived_from, detail) ||
        !references.reference_type_info(
            static_cast<asCTypeInfo*>(type.shadowType), encoded.shadow_type, detail)) {
        return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
    }

    const int behaviour_ids[7] = {
        type.beh.factory,
        type.beh.listFactory,
        type.beh.copyfactory,
        type.beh.construct,
        type.beh.copyconstruct,
        type.beh.destruct,
        type.beh.copy,
    };
    for (const int id : behaviour_ids) {
        if (!references.store_function_id(id, detail)) {
            return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
        }
        encoded.behaviour_references.push_back(id);
    }
    for (asUINT index = 0U; index < type.beh.constructors.GetLength(); ++index) {
        asCScriptFunction* function = engine.GetScriptFunction(type.beh.constructors[index]);
        if (function == nullptr) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "class constructor id does not resolve in the source engine", asERROR);
        }
        precompiled_function constructor;
        engine_bridge_result result = export_function(*function, &references, constructor);
        if (!result.succeeded()) {
            return result;
        }
        encoded.constructors.push_back(std::move(constructor));
    }
    for (asUINT index = 0U; index < type.beh.factories.GetLength(); ++index) {
        const int id = type.beh.factories[index];
        if (!references.store_function_id(id, detail)) {
            return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
        }
        encoded.factory_references.push_back(id);
    }
    if (type.beh.destruct != 0) {
        asCScriptFunction* function = engine.GetScriptFunction(type.beh.destruct);
        if (function == nullptr) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "class destructor id does not resolve in the source engine", asERROR);
        }
        precompiled_function destructor;
        engine_bridge_result result = export_function(*function, &references, destructor);
        if (!result.succeeded()) {
            return result;
        }
        encoded.behaviour_functions.push_back(std::move(destructor));
        encoded.behaviour_function_types.push_back(asBEHAVE_DESTRUCT);
    }

    if (!validate_class_shape(encoded, detail, false, true)) {
        return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
    }
    output = std::move(encoded);
    return {};
}

engine_bridge_result collect_class_generator_capabilities(
    asCModule& module,
    registry_runtime& registry,
    class_generator_capability_table& output) {
    class_generator_capability_table staged;
    staged.classes.reserve(module.classTypes.GetLength());
    for (asUINT type_index = 0U; type_index < module.classTypes.GetLength(); ++type_index) {
        const asCObjectType& type = *module.classTypes[type_index];
        class_generator_class_capabilities class_capabilities;
        class_capabilities.class_name = type.GetName();
        class_capabilities.name_space = type.GetNamespace();
        const int first_local_offset = type.derivedFrom != nullptr
            ? type.derivedFrom->size
            : type.shadowType != nullptr ? type.basePropertyOffset : 0;
        for (asUINT property_index = 0U;
             property_index < type.properties.GetLength(); ++property_index) {
            const asCObjectProperty& property = *type.properties[property_index];
            if (property.byteOffset < first_local_offset) continue;
            class_generator_type_capabilities resolved;
            std::string detail;
            const int type_id = module.engine->GetTypeIdFromDataType(property.type);
            const char* const type_declaration =
                module.engine->GetTypeDeclaration(type_id, true);
            if (type_declaration == nullptr || *type_declaration == '\0') {
                return failure(
                    engine_bridge_phase::export_module,
                    kNoModule,
                    "class-generator type declaration resolution failed for " +
                        class_capabilities.class_name + "::" +
                        property.name.AddressOf(),
                    asERROR);
            }
            if (!registry.resolve_class_generator_type_capabilities(
                    *module.engine, type_id, resolved, detail)) {
                return failure(
                    engine_bridge_phase::export_module,
                    kNoModule,
                    "class-generator capability resolution failed for " +
                        class_capabilities.class_name + "::" +
                        property.name.AddressOf() + ": " + detail,
                    asERROR);
            }
            class_capabilities.properties.push_back({
                property.name.AddressOf(),
                type_declaration,
                resolved.can_create_property,
                resolved.never_requires_gc,
                resolved.requires_property,
            });
        }
        staged.classes.push_back(std::move(class_capabilities));
    }
    output = std::move(staged);
    return {};
}

class deferred_build_settings final {
public:
    explicit deferred_build_settings(asCScriptEngine& engine) noexcept
        : engine_(engine),
          validation_(engine.deferValidationOfTemplateTypes),
          template_size_(engine.deferCalculatingTemplateSize) {
        engine_.deferValidationOfTemplateTypes = true;
        engine_.deferCalculatingTemplateSize = true;
    }

    ~deferred_build_settings() {
        engine_.deferValidationOfTemplateTypes = validation_;
        engine_.deferCalculatingTemplateSize = template_size_;
    }

private:
    asCScriptEngine& engine_;
    bool validation_;
    bool template_size_;
};

class source_builder_cleanup final {
public:
    void add(asCModule& module) { modules_.push_back(&module); }

    ~source_builder_cleanup() {
        for (asCModule* const module : modules_) {
            if (module->builder != nullptr) {
                asDELETE(module->builder, asCBuilder);
                module->builder = nullptr;
            }
        }
    }

private:
    std::vector<asCModule*> modules_;
};

struct mixed_module_state {
    asCModule* module = nullptr;
    const precompiled_module* cached = nullptr;
    const lexical_module_description* source = nullptr;
    std::size_t source_index = kNoModule;
};

asCTypeInfo* find_registered_type(
    asCScriptEngine& engine,
    const std::string& name) noexcept {
    return engine.allRegisteredTypesByName.FindFirst_CaseInsensitive(name.c_str());
}

bool preprocess_cached_inheritance(asCObjectType& type, std::string& detail) {
    if (type.derivedFrom != nullptr) {
        for (asUINT index = 0U; index < type.derivedFrom->properties.GetLength(); ++index) {
            asCObjectProperty* const property = type.derivedFrom->properties[index];
            if (property->byteOffset < type.basePropertyOffset) continue;
            type.properties.PushLast(property);
            type.propertyTable.Add(property);
        }
    }
    if ((type.flags & asOBJ_VALUE) != 0U) return true;
    for (asUINT slot = 0U; slot < type.virtualFunctionTable.GetLength(); ++slot) {
        if (type.virtualFunctionTable[slot] != nullptr) continue;
        asCScriptFunction* inherited = nullptr;
        for (asCObjectType* base = type.derivedFrom;
             base != nullptr; base = base->derivedFrom) {
            if (slot < base->virtualFunctionTable.GetLength() &&
                base->virtualFunctionTable[slot] != nullptr) {
                inherited = base->virtualFunctionTable[slot];
                break;
            }
        }
        if (inherited == nullptr) {
            detail = "class virtual slot has no implementation in its base chain";
            return false;
        }
        type.virtualFunctionTable[slot] = inherited;
        inherited->AddRefInternal();
        type.methods.PushLast(inherited->id);
        inherited->AddRefInternal();
        type.methodTable.Add(inherited);
    }
    return true;
}

// The stock source builder can recursively lay out other source classes, but
// it treats a precompiled shell (compilingDeclaration == nullptr) as complete.
// This coordinator therefore walks the combined dependency graph first and
// invokes the appropriate source or cache layout operation only after every
// base/value dependency has been completed.
class mixed_type_coordinator final {
public:
    mixed_type_coordinator(
        reference_resolver& references,
        asCScriptEngine& engine,
        const std::unordered_map<std::string, const native_super_type*>& native_by_path)
        : references_(references), engine_(engine), native_by_path_(native_by_path) {}

    void add_cached(
        asCObjectType& type,
        const precompiled_class& record,
        const std::size_t module_index) {
        cached_.emplace(&type, cached_location{&record, module_index});
        order_.push_back(&type);
    }

    void add_source(asCObjectType& type, const std::size_t module_index) {
        source_.emplace(&type, module_index);
        order_.push_back(&type);
    }

    bool link_cached(std::size_t& module_index, std::string& detail) {
        for (const auto& entry : cached_) {
            asCObjectType& type = *entry.first;
            const cached_location location = entry.second;
            module_index = location.module_index;
            type.derivedFrom = nullptr;
            if (location.record->derived_from != 0) {
                asCTypeInfo* base_info = nullptr;
                if (!references_.get_type_info(
                        location.record->derived_from, base_info, false, detail) ||
                    base_info == nullptr) {
                    return false;
                }
                type.derivedFrom = CastToObjectType(base_info);
                if (type.derivedFrom == nullptr ||
                    (type.derivedFrom->flags & asOBJ_SCRIPT_OBJECT) == 0U) {
                    detail = "saved script base reference does not resolve to a script object type";
                    return false;
                }
            }
            type.shadowType = nullptr;
            type.basePropertyOffset = 0;
            if (location.record->shadow_type != 0) {
                asCTypeInfo* shadow = nullptr;
                if (!references_.get_type_info(
                        location.record->shadow_type, shadow, false, detail) ||
                    shadow == nullptr) {
                    return false;
                }
                const auto native = native_by_path_.find(
                    location.record->code_super_class.bytes);
                if (native == native_by_path_.end() ||
                    native->second->property_offset >
                        static_cast<std::uint64_t>(std::numeric_limits<int>::max()) ||
                    find_registered_type(
                        engine_, native->second->angelscript_type_name) != shadow) {
                    detail = "saved shadow type does not match the sealed native-super profile";
                    return false;
                }
                type.shadowType = shadow;
                type.basePropertyOffset =
                    static_cast<int>(native->second->property_offset);
            }
        }
        return true;
    }

    bool process_all(std::size_t& module_index, std::string& detail) {
        for (asCObjectType* const type : order_) {
            if (!process(*type, module_index, detail)) return false;
        }
        return true;
    }

private:
    struct cached_location {
        const precompiled_class* record = nullptr;
        std::size_t module_index = kNoModule;
    };

    bool process_dependency(
        asCTypeInfo* const dependency,
        std::size_t& module_index,
        std::string& detail) {
        if (dependency == nullptr) return true;
        auto* const object = CastToObjectType(dependency);
        if (object == nullptr) return true;
        if ((object->flags & asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE) != 0U) {
            if (object->templateSubTypes.GetLength() != 0U) {
                asCTypeInfo* const subtype = object->templateSubTypes[0].GetTypeInfo();
                if (subtype != nullptr && (subtype->flags & asOBJ_REF) == 0U &&
                    !process_dependency(subtype, module_index, detail)) {
                    return false;
                }
            }
            object->CalculateTemplateSize();
            return true;
        }
        if ((object->flags & asOBJ_SCRIPT_OBJECT) == 0U || object->size != -1) {
            return true;
        }
        if (cached_.find(object) == cached_.end() &&
            source_.find(object) == source_.end()) {
            detail = "script layout dependency is not owned by the final mixed graph";
            return false;
        }
        return process(*object, module_index, detail);
    }

    bool process(
        asCObjectType& type,
        std::size_t& module_index,
        std::string& detail) {
        const auto state = states_.find(&type);
        if (state != states_.end()) {
            if (state->second == 2U) return true;
            detail = "cycle detected in mixed script layout dependencies";
            return false;
        }
        states_.emplace(&type, 1U);

        const auto cached = cached_.find(&type);
        const auto source = source_.find(&type);
        if (cached == cached_.end() && source == source_.end()) {
            detail = "mixed layout requested an unowned script type";
            return false;
        }
        module_index = cached != cached_.end()
            ? cached->second.module_index
            : source->second;

        if (!process_dependency(type.derivedFrom, module_index, detail)) return false;

        if (cached != cached_.end()) {
            for (const precompiled_property& property : cached->second.record->properties) {
                if (property.type.type_info == 0) continue;
                asCTypeInfo* property_type = nullptr;
                if (!references_.get_type_info(
                        property.type.type_info, property_type, false, detail)) {
                    return false;
                }
                if ((property_type->flags & asOBJ_VALUE) != 0U &&
                    !process_dependency(property_type, module_index, detail)) {
                    return false;
                }
            }
            if (!create_class_properties(
                    type, *cached->second.record, references_, detail) ||
                !preprocess_cached_inheritance(type, detail)) {
                return false;
            }
        } else {
            for (asUINT index = 0U; index < type.properties.GetLength(); ++index) {
                asCTypeInfo* const property_type =
                    type.properties[index]->type.GetTypeInfo();
                if (property_type != nullptr &&
                    (property_type->flags & asOBJ_VALUE) != 0U &&
                    !process_dependency(property_type, module_index, detail)) {
                    return false;
                }
            }
            if (type.module == nullptr || type.module->builder == nullptr ||
                !type.module->builder->EnsureClassLayouted(&type) || type.size < 0) {
                detail = "source builder failed to lay out a mixed-graph script type";
                return false;
            }
        }
        states_[&type] = 2U;
        return true;
    }

    reference_resolver& references_;
    asCScriptEngine& engine_;
    const std::unordered_map<std::string, const native_super_type*>& native_by_path_;
    std::unordered_map<asCObjectType*, cached_location> cached_;
    std::unordered_map<asCObjectType*, std::size_t> source_;
    std::unordered_map<asCObjectType*, std::uint8_t> states_;
    std::vector<asCObjectType*> order_;
};

} // namespace

engine_bridge_result rehydrate_cache_checkpoint(
    asIScriptEngine& engine_interface,
    const cache& input,
    std::vector<asIScriptModule*>& modules) {
    try {
        auto& engine = static_cast<asCScriptEngine&>(engine_interface);
        std::vector<std::string> module_names;
        engine_bridge_result result = preflight_cache(engine, input, module_names);
        if (!result.succeeded()) {
            return result;
        }
        reference_resolver references(engine, input);
        class_layout_replayer class_layouts(references);
        class_function_preprocessor class_functions;

        module_cleanup cleanup(engine);
        cleanup.reserve(module_names.size());
        build_guard guard(engine);
        result.code = guard.request();
        if (result.code < 0) {
            result.phase = engine_bridge_phase::request_build;
            return result;
        }
        engine.PrepareEngine();
        if (engine.configFailed) {
            return failure(
                engine_bridge_phase::prepare_engine, kNoModule,
                "engine configuration failed", asINVALID_CONFIGURATION);
        }

        std::vector<asCModule*> created;
        created.reserve(input.modules.size());
        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            auto* module = static_cast<asCModule*>(
                engine.GetModule(module_names[index].c_str(), asGM_ALWAYS_CREATE));
            if (module == nullptr) {
                result = failure(
                    engine_bridge_phase::create_modules, index,
                    "engine refused to create a precompiled module", asERROR);
                break;
            }
            module->baseModuleName = module_names[index].c_str();
            created.push_back(module);
            cleanup.add(module_names[index]);
            for (const precompiled_class& type_record : input.modules[index].second.classes) {
                asCObjectType* type = create_class_shell(engine, *module, type_record);
                class_layouts.add(*type, type_record, index);
                class_functions.add(*type, type_record, index);
            }
            for (const precompiled_enum& enumeration : input.modules[index].second.enums) {
                create_enum(engine, *module, enumeration);
            }
        }

        if (result.succeeded()) {
            for (std::size_t index = 0U; index < created.size(); ++index) {
                for (const archive_string& imported : input.modules[index].second.imported_modules) {
                    asIScriptModule* dependency =
                        engine.GetModule(imported.bytes.c_str(), false);
                    if (dependency == nullptr) {
                        result = failure(
                            engine_bridge_phase::create_modules, index,
                            "saved imported module is absent from the target engine", asERROR);
                        break;
                    }
                    created[index]->ImportModule(dependency);
                }
                if (!result.succeeded()) {
                    break;
                }
            }
        }

        if (result.succeeded()) {
            std::size_t failed_module = kNoModule;
            std::string detail;
            if (!class_layouts.process_all(failed_module, detail)) {
                result = failure(
                    engine_bridge_phase::create_types,
                    failed_module, std::move(detail), asERROR);
            }
        }

        if (result.succeeded()) {
            for (std::size_t index = 0U; index < input.modules.size(); ++index) {
                asCModule& module = *created[index];
                const precompiled_module& record = input.modules[index].second;
                const int script_section_index = engine.GetScriptSectionNameIndex(
                    record.script_relative_filename.bytes.c_str());
                for (const function_import& imported : record.function_imports) {
                    std::string detail;
                    if (!add_function_import(
                            engine, module, imported, references, detail)) {
                        result = failure(
                            engine_bridge_phase::create_globals_and_functions,
                            index, std::move(detail), asERROR);
                        break;
                    }
                }
                if (!result.succeeded()) {
                    break;
                }
                for (const precompiled_global& global : record.global_variables) {
                    asCGlobalProperty* property = nullptr;
                    std::string detail;
                    if (!create_global(
                            engine, module, global, script_section_index,
                            references, property, detail)) {
                        result = failure(
                            engine_bridge_phase::create_globals_and_functions,
                            index, std::move(detail), asERROR);
                        break;
                    }
                }
                if (!result.succeeded()) {
                    break;
                }
                for (const precompiled_function& function_record : record.functions) {
                    asCScriptFunction* function = nullptr;
                    std::string detail;
                    if (!create_function(
                            engine, module, function_record, script_section_index,
                            references, function, detail)) {
                        result = failure(
                            engine_bridge_phase::create_globals_and_functions,
                            index, std::move(detail), asERROR);
                        break;
                    }
                    function->id = engine.GetNextScriptFunctionId();
                    function->CalculateParameterOffsets();
                    module.AddScriptFunction(function);
                    module.globalFunctions.Add(function);
                    module.globalFunctionList.PushLast(function);
                }
                if (!result.succeeded()) {
                    break;
                }
            }
        }

        if (result.succeeded()) {
            for (std::size_t module_index = 0U;
                 module_index < input.modules.size(); ++module_index) {
                asCModule& module = *created[module_index];
                const precompiled_module& record = input.modules[module_index].second;
                const int script_section_index = engine.GetScriptSectionNameIndex(
                    record.script_relative_filename.bytes.c_str());
                for (std::size_t class_index = 0U;
                     class_index < record.classes.size(); ++class_index) {
                    std::string detail;
                    if (!create_class_functions(
                            engine, module,
                            *module.classTypes[static_cast<asUINT>(class_index)],
                            record.classes[class_index], script_section_index,
                            references, detail)) {
                        result = failure(
                            engine_bridge_phase::create_globals_and_functions,
                            module_index, std::move(detail), asERROR);
                        break;
                    }
                }
                if (!result.succeeded()) {
                    break;
                }
            }
        }

        if (result.succeeded()) {
            for (std::size_t module_index = 0U;
                 module_index < input.modules.size(); ++module_index) {
                asCModule& module = *created[module_index];
                const precompiled_module& record = input.modules[module_index].second;
                for (std::size_t class_index = 0U;
                     class_index < record.classes.size(); ++class_index) {
                    std::string detail;
                    if (!bind_class_function_references(
                            *module.classTypes[static_cast<asUINT>(class_index)],
                            record.classes[class_index], references, detail)) {
                        result = failure(
                            engine_bridge_phase::create_globals_and_functions,
                            module_index, std::move(detail), asERROR);
                        break;
                    }
                }
                if (!result.succeeded()) {
                    break;
                }
            }
        }

        if (result.succeeded()) {
            std::size_t failed_module = kNoModule;
            std::string detail;
            if (!class_functions.process_all(failed_module, detail)) {
                result = failure(
                    engine_bridge_phase::create_globals_and_functions,
                    failed_module, std::move(detail), asERROR);
            }
        }

        if (result.succeeded()) {
            for (std::size_t index = 0U; index < created.size(); ++index) {
                asCModule& module = *created[index];
                for (asUINT function_index = 0U;
                     function_index < module.scriptFunctions.GetLength(); ++function_index) {
                    std::string detail;
                    if (!references.relocate_function(
                            *module.scriptFunctions[function_index], detail)) {
                        result = failure(
                            engine_bridge_phase::relocate_bytecode,
                            index, std::move(detail), asERROR);
                        break;
                    }
                }
                for (asUINT global_index = 0U;
                     result.succeeded() && global_index < module.scriptGlobalsList.GetLength();
                     ++global_index) {
                    asCScriptFunction* initializer =
                        module.scriptGlobalsList[global_index]->GetInitFunc();
                    if (initializer == nullptr) {
                        continue;
                    }
                    std::string detail;
                    if (!references.relocate_function(*initializer, detail)) {
                        result = failure(
                            engine_bridge_phase::relocate_bytecode,
                            index, std::move(detail), asERROR);
                    }
                }
                if (!result.succeeded()) {
                    break;
                }
            }
        }

        if (result.succeeded()) {
            for (std::size_t index = 0U; index < created.size(); ++index) {
                const int reset = created[index]->ResetGlobalVars(nullptr);
                if (reset < 0) {
                    result = failure(
                        engine_bridge_phase::initialize_globals, index,
                        "precompiled module global initialization failed", reset);
                    break;
                }
            }
        }

        if (!result.succeeded()) {
            return result;
        }
        std::vector<asIScriptModule*> loaded;
        loaded.reserve(created.size());
        for (asCModule* module : created) {
            loaded.push_back(module);
        }
        modules = std::move(loaded);
        cleanup.keep();
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed in precompiled engine bridge", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected precompiled engine bridge failure", asERROR);
    }
}

engine_bridge_result compile_mixed_cache_checkpoint(
    asIScriptEngine& engine_interface,
    const cache& base,
    const preprocessor_options& options,
    const lexical_preprocess_result& source,
    registry_runtime* const registry,
    frontend_compile_runtime& frontend_runtime,
    const bool initialize_source_globals,
    std::vector<asIScriptModule*>& modules,
    shipping_static_jit_candidates* const static_jit_candidates) {
    try {
        auto& engine = static_cast<asCScriptEngine&>(engine_interface);
        std::vector<std::string> cache_names;
        engine_bridge_result result = preflight_cache(
            engine, base, cache_names, true);
        if (!result.succeeded()) return result;
        if (!source.ok || source.modules.size() > max_preprocessor_sources ||
            std::any_of(
                source.diagnostics.begin(), source.diagnostics.end(),
                [](const preprocessor_diagnostic& diagnostic) {
                    return diagnostic.severity == preprocessor_diagnostic_severity::error;
                })) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "mixed frontend input is invalid or contains preprocessing errors",
                asINVALID_ARG);
        }

        std::unordered_map<std::string, std::size_t> cache_by_name;
        cache_by_name.reserve(cache_names.size());
        for (std::size_t index = 0U; index < cache_names.size(); ++index) {
            cache_by_name.emplace(cache_names[index], index);
        }
        std::unordered_map<std::string, std::size_t> source_by_name;
        source_by_name.reserve(source.modules.size());
        std::unordered_map<std::string, const native_super_type*> native_by_path;
        native_by_path.reserve(options.native_super_types.size());
        for (const native_super_type& native : options.native_super_types) {
            if (native.unreal_class_path.empty() ||
                !native_by_path.emplace(native.unreal_class_path, &native).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "native-super profile has an empty or duplicate Unreal class path",
                    asINVALID_CONFIGURATION);
            }
        }

        std::vector<std::string> final_names = cache_names;
        final_names.reserve(cache_names.size() + source.modules.size());
        std::vector<std::size_t> source_final_indices(
            source.modules.size(), kNoModule);
        for (std::size_t index = 0U; index < source.modules.size(); ++index) {
            const lexical_module_description& description = source.modules[index];
            if (description.module_name.empty() ||
                !source_by_name.emplace(description.module_name, index).second) {
                return failure(
                    engine_bridge_phase::preflight, index,
                    "mixed frontend contains an empty or duplicate module name",
                    asINVALID_ARG);
            }
            const auto cached = cache_by_name.find(description.module_name);
            if (cached != cache_by_name.end()) {
                source_final_indices[index] = cached->second;
            } else {
                if (engine.GetModule(description.module_name.c_str(), false) != nullptr) {
                    return failure(
                        engine_bridge_phase::preflight, index,
                        "added source module already exists in the target engine",
                        asALREADY_REGISTERED);
                }
                source_final_indices[index] = final_names.size();
                final_names.push_back(description.module_name);
            }
            if (!description.delegates.empty() && registry == nullptr) {
                return failure(
                    engine_bridge_phase::preflight, index,
                    "delegate compilation requires the live registry runtime",
                    asINVALID_CONFIGURATION);
            }
            for (const preprocessed_class_description& type : description.classes) {
                if (type.is_struct || type.code_super_class.empty()) continue;
                const auto native = native_by_path.find(type.code_super_class);
                if (native == native_by_path.end() ||
                    find_registered_type(
                        engine, native->second->angelscript_type_name) == nullptr) {
                    return failure(
                        engine_bridge_phase::preflight, index,
                        "source class has an unavailable profiled native superclass",
                        asINVALID_CONFIGURATION);
                }
            }
        }

        std::unordered_set<std::string> final_name_set(
            final_names.begin(), final_names.end());
        for (std::size_t index = 0U; index < source.modules.size(); ++index) {
            for (const std::string& imported : source.modules[index].imported_modules) {
                if (final_name_set.find(imported) == final_name_set.end() &&
                    engine.GetModule(imported.c_str(), false) == nullptr) {
                    return failure(
                        engine_bridge_phase::preflight, index,
                        "source import is absent from the final mixed graph",
                        asNO_MODULE);
                }
            }
        }
        if (registry == nullptr && std::any_of(
                base.modules.begin(), base.modules.end(),
                [](const auto& entry) {
                    return !entry.second.declared_events.empty() ||
                           !entry.second.declared_delegates.empty();
                })) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "cached delegate restoration requires the live registry runtime",
                asINVALID_CONFIGURATION);
        }

        reference_resolver references(engine, base);
        mixed_type_coordinator types(references, engine, native_by_path);
        module_cleanup cleanup(engine);
        cleanup.reserve(final_names.size());
        build_guard guard(engine);
        result.code = guard.request();
        if (result.code < 0) {
            result.phase = engine_bridge_phase::request_build;
            return result;
        }
        deferred_build_settings deferred(engine);
        engine.PrepareEngine();
        if (engine.configFailed) {
            return failure(
                engine_bridge_phase::prepare_engine, kNoModule,
                "engine configuration failed", asINVALID_CONFIGURATION);
        }

        std::vector<mixed_module_state> states(final_names.size());
        for (std::size_t index = 0U; index < final_names.size(); ++index) {
            auto* const module = static_cast<asCModule*>(
                engine.GetModule(final_names[index].c_str(), asGM_ALWAYS_CREATE));
            if (module == nullptr) {
                return failure(
                    engine_bridge_phase::create_modules, index,
                    "engine refused to create a mixed-graph module", asERROR);
            }
            module->baseModuleName = final_names[index].c_str();
            states[index].module = module;
            cleanup.add(final_names[index]);
        }
        for (std::size_t source_index = 0U;
             source_index < source.modules.size(); ++source_index) {
            mixed_module_state& state = states[source_final_indices[source_index]];
            state.source = &source.modules[source_index];
            state.source_index = source_index;
        }
        for (std::size_t index = 0U; index < base.modules.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source != nullptr) continue;
            state.cached = &base.modules[index].second;
            for (const precompiled_class& record : state.cached->classes) {
                asCObjectType* const type = create_class_shell(engine, *state.module, record);
                types.add_cached(*type, record, index);
            }
            for (const precompiled_enum& record : state.cached->enums) {
                create_enum(engine, *state.module, record);
            }
            const auto classify_cached = [&](
                const std::vector<archive_string>& names,
                const bool multicast) -> bool {
                for (const archive_string& name : names) {
                    asCObjectType* const type = CastToObjectType(
                        state.module->allLocalTypes.FindFirst(name.bytes.c_str()));
                    void* const tag = frontend_runtime.delegate_tag(multicast);
                    if (type == nullptr || tag == nullptr) return false;
                    type->plainUserData = reinterpret_cast<asPWORD>(tag);
                    if (!classify_dynamic_script_type(
                            *registry, *type,
                            multicast
                                ? dynamic_script_type_category::multicast_delegate
                                : dynamic_script_type_category::delegate)) {
                        return false;
                    }
                }
                return true;
            };
            if (!classify_cached(state.cached->declared_events, true) ||
                !classify_cached(state.cached->declared_delegates, false)) {
                return failure(
                    engine_bridge_phase::create_types, index,
                    "cached delegate/event tag could not be restored", asERROR);
            }
        }

        source_builder_cleanup builder_cleanup;
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr) continue;
            builder_cleanup.add(*state.module);
            for (const preprocessed_class_description& type : state.source->classes) {
                if (type.is_struct || type.code_super_class.empty()) continue;
                const native_super_type& native = *native_by_path.at(type.code_super_class);
                asPreClassData data;
                data.PropertyOffset = static_cast<std::size_t>(native.property_offset);
                data.ShadowType = find_registered_type(engine, native.angelscript_type_name);
                state.module->AddPreClassData(type.class_name.c_str(), data);
            }
            for (const preprocessed_delegate_description& type : state.source->delegates) {
                asPreClassData data;
                data.InitialUserData = frontend_runtime.delegate_tag(type.multicast);
                if (data.InitialUserData == nullptr) {
                    return failure(
                        engine_bridge_phase::create_types, index,
                        "frontend runtime was moved from", asINVALID_ARG);
                }
                state.module->AddPreClassData(type.delegate_name.c_str(), data);
            }
            for (const preprocessed_code_section& section : state.source->code) {
                const int added = state.module->AddScriptSection(
                    section.absolute_path.c_str(), section.conditioned_code.data(),
                    section.conditioned_code.size(), 0);
                if (added < 0) {
                    return failure(
                        engine_bridge_phase::create_modules, index,
                        "engine rejected a mixed source section", added);
                }
            }
            if (!state.source->editor_only_blocks.empty() &&
                state.source->code.size() != 1U) {
                return failure(
                    engine_bridge_phase::create_modules, index,
                    "editor-only line blocks require exactly one source section",
                    asINVALID_CONFIGURATION);
            }
            TArray<TPair<int, int>> editor_blocks;
            for (const editor_only_line_block& block :
                 state.source->editor_only_blocks) {
                const std::uint32_t open_end =
                    (std::numeric_limits<std::uint32_t>::max)();
                if (block.first_line == 0U ||
                    block.first_line > static_cast<std::uint32_t>(
                        (std::numeric_limits<int>::max)()) ||
                    (block.last_line != open_end &&
                     (block.last_line < block.first_line ||
                      block.last_line > static_cast<std::uint32_t>(
                          (std::numeric_limits<int>::max)())))) {
                    return failure(
                        engine_bridge_phase::create_modules, index,
                        "source module has an invalid editor-only line block",
                        asINVALID_ARG);
                }
                editor_blocks.Emplace(
                    static_cast<int>(block.first_line),
                    block.last_line == open_end
                        ? -1
                        : static_cast<int>(block.last_line));
            }
            state.module->builder->SetEditorOnlyBlockLinePositions(editor_blocks);
            state.module->builder->isEditorOnlyModule =
                is_editor_only_module_name(state.source->module_name);
        }

        for (std::size_t index = 0U; index < states.size(); ++index) {
            const auto import_one = [&](const std::string& imported) -> bool {
                asIScriptModule* const dependency =
                    engine.GetModule(imported.c_str(), false);
                if (dependency == nullptr) return false;
                states[index].module->ImportModule(dependency);
                return true;
            };
            if (states[index].cached != nullptr) {
                for (const archive_string& imported :
                     states[index].cached->imported_modules) {
                    if (!import_one(imported.bytes)) {
                        return failure(
                            engine_bridge_phase::create_modules, index,
                            "cached import is absent from the final mixed graph", asNO_MODULE);
                    }
                }
            } else {
                std::unordered_set<std::string> imported_modules;
                for (const std::string& imported : states[index].source->imported_modules) {
                    if (!imported_modules.insert(imported).second) continue;
                    if (!import_one(imported)) {
                        return failure(
                            engine_bridge_phase::create_modules, index,
                            "source import disappeared after mixed preflight", asNO_MODULE);
                    }
                }
                // With AutomaticImports enabled the donor compiles loose source against the
                // complete existing script graph. Cached modules are recreated as module-local
                // shells in this bridge, so publish them to the authored module explicitly before
                // Stage 1 type generation. These bridge-only imports are not serialized into the
                // output module metadata; they only reproduce the donor's compile-time namespace.
                if (options.automatic_imports) {
                    for (std::size_t dependency = 0U;
                         dependency < states.size(); ++dependency) {
                        if (dependency == index || states[dependency].cached == nullptr ||
                            !imported_modules.insert(final_names[dependency]).second) {
                            continue;
                        }
                        if (!import_one(final_names[dependency])) {
                            return failure(
                                engine_bridge_phase::create_modules, index,
                                "automatic cached-module import disappeared during mixed build",
                                asNO_MODULE);
                        }
                    }
                }
            }
        }

        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            state.module->InternalReset();
            const int code = state.module->builder->BuildParallelParseScripts();
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::parse_source, index,
                    "source parse failed in mixed graph", code);
            }
        }
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            const int code = state.module->builder->BuildGenerateTypes();
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::generate_source_types, index,
                    "source type generation failed in mixed graph", code);
            }
            for (asUINT type_index = 0U;
                 type_index < state.module->classTypes.GetLength(); ++type_index) {
                types.add_source(*state.module->classTypes[type_index], index);
            }
            for (const preprocessed_delegate_description& delegate :
                 state.source->delegates) {
                const std::string declaration = delegate.name_space.empty()
                    ? delegate.delegate_name
                    : delegate.name_space + "::" + delegate.delegate_name;
                asITypeInfo* const type =
                    state.module->GetTypeInfoByDecl(declaration.c_str());
                if (type == nullptr || registry == nullptr ||
                    !classify_dynamic_script_type(
                        *registry, *type,
                        delegate.multicast
                            ? dynamic_script_type_category::multicast_delegate
                            : dynamic_script_type_category::delegate)) {
                    return failure(
                        engine_bridge_phase::generate_source_types, index,
                        "source delegate classification failed in mixed graph", asERROR);
                }
            }
        }

        std::size_t failed_module = kNoModule;
        std::string detail;
        if (!types.link_cached(failed_module, detail)) {
            return failure(
                engine_bridge_phase::create_types, failed_module,
                std::move(detail), asERROR);
        }

        // Publish every cached function declaration before source Stage 2 so
        // added/edited modules can compile declarations against unchanged
        // precompiled providers regardless of final module order.
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.cached == nullptr) continue;
            const int section = engine.GetScriptSectionNameIndex(
                state.cached->script_relative_filename.bytes.c_str());
            for (const function_import& imported : state.cached->function_imports) {
                detail.clear();
                if (!add_function_import(engine, *state.module, imported, references, detail)) {
                    return failure(
                        engine_bridge_phase::create_globals_and_functions, index,
                        std::move(detail), asERROR);
                }
            }
            for (const precompiled_function& record : state.cached->functions) {
                asCScriptFunction* function = nullptr;
                detail.clear();
                if (!create_function(
                        engine, *state.module, record, section,
                        references, function, detail)) {
                    return failure(
                        engine_bridge_phase::create_globals_and_functions, index,
                        std::move(detail), asERROR);
                }
                function->id = engine.GetNextScriptFunctionId();
                function->CalculateParameterOffsets();
                state.module->AddScriptFunction(function);
                state.module->globalFunctions.Add(function);
                state.module->globalFunctionList.PushLast(function);
            }
            for (std::size_t class_index = 0U;
                 class_index < state.cached->classes.size(); ++class_index) {
                detail.clear();
                if (!create_class_functions(
                        engine, *state.module,
                        *state.module->classTypes[static_cast<asUINT>(class_index)],
                        state.cached->classes[class_index], section,
                        references, detail)) {
                    return failure(
                        engine_bridge_phase::create_globals_and_functions, index,
                        std::move(detail), asERROR);
                }
            }
        }
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            const int code = state.module->builder->BuildGenerateFunctions();
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::generate_source_functions, index,
                    "source function generation failed in mixed graph", code);
            }
        }

        failed_module = kNoModule;
        detail.clear();
        if (!types.process_all(failed_module, detail)) {
            return failure(
                engine_bridge_phase::layout_types, failed_module,
                std::move(detail), asERROR);
        }
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            const int code = state.module->builder->BuildLayoutClasses();
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::layout_types, index,
                    "source class layout finalization failed in mixed graph", code);
            }
        }

        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.cached == nullptr) continue;
            const int section = engine.GetScriptSectionNameIndex(
                state.cached->script_relative_filename.bytes.c_str());
            for (const precompiled_global& global : state.cached->global_variables) {
                asCGlobalProperty* property = nullptr;
                detail.clear();
                if (!create_global(
                        engine, *state.module, global, section,
                        references, property, detail)) {
                    return failure(
                        engine_bridge_phase::create_globals_and_functions, index,
                        std::move(detail), asERROR);
                }
            }
            for (std::size_t class_index = 0U;
                 class_index < state.cached->classes.size(); ++class_index) {
                detail.clear();
                if (!bind_class_function_references(
                        *state.module->classTypes[static_cast<asUINT>(class_index)],
                        state.cached->classes[class_index], references, detail)) {
                    return failure(
                        engine_bridge_phase::create_globals_and_functions, index,
                        std::move(detail), asERROR);
                }
            }
        }

        engine.deferCalculatingTemplateSize = false;
        for (asCObjectType* instance : engine.unvalidatedTemplateInstances) {
            instance->CalculateTemplateSize();
        }
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            const int code = state.module->builder->BuildLayoutFunctions();
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::layout_source_functions, index,
                    "source function layout failed in mixed graph", code);
            }
        }

        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.cached == nullptr) continue;
            for (asUINT function_index = 0U;
                 function_index < state.module->scriptFunctions.GetLength();
                 ++function_index) {
                detail.clear();
                if (!references.relocate_function(
                        *state.module->scriptFunctions[function_index], detail)) {
                    return failure(
                        engine_bridge_phase::relocate_bytecode, index,
                        std::move(detail), asERROR);
                }
            }
            for (asUINT global_index = 0U;
                 global_index < state.module->scriptGlobalsList.GetLength();
                 ++global_index) {
                asCScriptFunction* const initializer =
                    state.module->scriptGlobalsList[global_index]->GetInitFunc();
                if (initializer == nullptr) continue;
                detail.clear();
                if (!references.relocate_function(*initializer, detail)) {
                    return failure(
                        engine_bridge_phase::relocate_bytecode, index,
                        std::move(detail), asERROR);
                }
            }
        }
        if (static_jit_candidates != nullptr) {
            static_jit_candidates->functions.clear();
        }
        for (std::size_t index = 0U; index < states.size(); ++index) {
            mixed_module_state& state = states[index];
            if (state.source == nullptr || state.module->builder == nullptr) continue;
            const int code = state.module->builder->BuildCompileCode();
            asDELETE(state.module->builder, asCBuilder);
            state.module->builder = nullptr;
            if (code != asSUCCESS) {
                return failure(
                    engine_bridge_phase::compile_source_code, index,
                    "source bytecode compilation failed in mixed graph", code);
            }
            state.module->JITCompile();
            if (static_jit_candidates != nullptr) {
                for (asUINT function_index = 0U;
                     function_index < state.module->scriptFunctions.GetLength();
                     ++function_index) {
                    asCScriptFunction* const function =
                        state.module->scriptFunctions[function_index];
                    if (function != nullptr && function->funcType == asFUNC_SCRIPT) {
                        static_jit_candidates->functions.push_back(function);
                    }
                }
            }
        }

        asCBuilder validator(&engine, nullptr);
        validator.Reset();
        validator.EvaluateTemplateInstances(false);
        engine.deferValidationOfTemplateTypes = false;
        if (validator.numErrors > 0) {
            return failure(
                engine_bridge_phase::validate_template_instances, kNoModule,
                "mixed graph contains invalid template instances", asERROR);
        }
        if (initialize_source_globals) {
            for (std::size_t index = 0U; index < states.size(); ++index) {
                // Cached modules are compiler inputs, not a replacement game
                // runtime. Their initializers may call host APIs whose real
                // implementations and world state intentionally do not exist
                // in the standalone process. Qualification adapters opt in
                // for newly compiled probe modules because they execute probe
                // entry points in this process. Product compilation keeps the
                // compiled initializer bytecode for the game runtime instead.
                if (states[index].source == nullptr) continue;
                const int code = states[index].module->ResetGlobalVars(nullptr);
                if (code != asSUCCESS) {
                    return failure(
                        engine_bridge_phase::initialize_globals, index,
                        "mixed module global initialization failed", code);
                }
            }
        }

        std::vector<asIScriptModule*> built;
        built.reserve(states.size());
        for (const mixed_module_state& state : states) built.push_back(state.module);
        modules = std::move(built);
        cleanup.keep();
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed in mixed cache/source compiler", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected mixed cache/source compiler failure", asERROR);
    }
}

engine_bridge_result apply_shipping_static_jit_checkpoint(
    const std::vector<asIScriptModule*>& modules,
    const shipping_static_jit_candidates& candidates) {
    try {
        std::unordered_set<asIScriptModule*> graph_modules;
        graph_modules.reserve(modules.size());
        asIScriptEngine* graph_engine = nullptr;
        for (asIScriptModule* const module_interface : modules) {
            if (module_interface == nullptr ||
                !graph_modules.insert(module_interface).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT analysis received a null or duplicate module",
                    asINVALID_ARG);
            }
            asIScriptEngine* const module_engine = module_interface->GetEngine();
            if (module_engine == nullptr ||
                (graph_engine != nullptr && module_engine != graph_engine)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT modules do not belong to one engine graph",
                    asINVALID_ARG);
            }
            graph_engine = module_engine;
        }
        if (graph_engine == nullptr) {
            if (!candidates.functions.empty()) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidates require a non-empty module graph",
                    asINVALID_ARG);
            }
            return {};
        }
        if (graph_engine->GetModuleCount() != graph_modules.size()) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "Shipping StaticJIT analysis requires the complete engine module graph",
                asINVALID_CONFIGURATION);
        }
        for (asUINT index = 0U; index < graph_engine->GetModuleCount(); ++index) {
            asIScriptModule* const engine_module = graph_engine->GetModuleByIndex(index);
            if (engine_module == nullptr ||
                graph_modules.find(engine_module) == graph_modules.end()) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT analysis requires the complete engine module graph",
                    asINVALID_CONFIGURATION);
            }
        }

        std::unordered_set<asCScriptFunction*> graph_script_functions;
        for (asIScriptModule* const module_interface : modules) {
            auto& module = static_cast<asCModule&>(*module_interface);
            for (asUINT index = 0U; index < module.scriptFunctions.GetLength(); ++index) {
                asCScriptFunction* const function = module.scriptFunctions[index];
                if (function != nullptr && function->funcType == asFUNC_SCRIPT) {
                    graph_script_functions.insert(function);
                }
            }
        }
        std::unordered_set<asCScriptFunction*> candidate_functions;
        candidate_functions.reserve(candidates.functions.size());
        for (asIScriptFunction* const candidate_interface : candidates.functions) {
            if (candidate_interface == nullptr ||
                candidate_interface->GetFuncType() != asFUNC_SCRIPT) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidate set is invalid", asINVALID_ARG);
            }
            asCScriptFunction* const candidate =
                static_cast<asCScriptFunction*>(candidate_interface);
            if (graph_script_functions.find(candidate) == graph_script_functions.end()) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidate is absent from the engine module graph",
                    asINVALID_ARG);
            }
            candidate_functions.insert(candidate);
        }
        std::unordered_set<asCScriptFunction*> functions_with_virtual_overrides;
        for (asIScriptModule* const module_interface : modules) {
            auto& module = static_cast<asCModule&>(*module_interface);
            for (asUINT type_index = 0U; type_index < module.classTypes.GetLength(); ++type_index) {
                asCObjectType* const object_type = module.classTypes[type_index];
                if (object_type == nullptr) continue;
                for (asUINT slot = 0U; slot < object_type->virtualFunctionTable.GetLength(); ++slot) {
                    asCScriptFunction* const function = object_type->virtualFunctionTable[slot];
                    if (function == nullptr || function->vfTableIdx == -1 ||
                        function->objectType != object_type) {
                        continue;
                    }
                    for (asCObjectType* base = object_type->derivedFrom; base != nullptr;) {
                        if (function->vfTableIdx >=
                            static_cast<int>(base->virtualFunctionTable.GetLength())) {
                            break;
                        }
                        asCScriptFunction* const base_function =
                            base->virtualFunctionTable[function->vfTableIdx];
                        if (base_function == nullptr || base_function->objectType == nullptr) break;
                        functions_with_virtual_overrides.insert(base_function);
                        base = base_function->objectType->derivedFrom;
                    }
                }
            }
        }
        for (asCScriptFunction* const candidate : candidate_functions) {
            if (functions_with_virtual_overrides.find(candidate) !=
                    functions_with_virtual_overrides.end() &&
                candidate->traits.GetTrait(asTRAIT_FINAL)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT override candidate is already final",
                    asINVALID_CONFIGURATION);
            }
        }
        for (asIScriptModule* const module_interface : modules) {
            auto& module = static_cast<asCModule&>(*module_interface);
            for (asUINT index = 0U; index < module.scriptFunctions.GetLength(); ++index) {
                asCScriptFunction* const function = module.scriptFunctions[index];
                if (function == nullptr || function->funcType != asFUNC_SCRIPT ||
                    candidate_functions.find(function) == candidate_functions.end()) {
                    continue;
                }
                if (functions_with_virtual_overrides.find(function) ==
                    functions_with_virtual_overrides.end()) {
                    function->traits.SetTrait(asTRAIT_FINAL, true);
                }
            }
        }
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed during Shipping StaticJIT analysis", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected Shipping StaticJIT analysis failure", asERROR);
    }
}

namespace {

bool static_jit_function_identity(
    asIScriptFunction& function,
    std::pair<std::string, std::string>& identity) {
    const char* const module_name = function.GetModuleName();
    const char* const declaration =
        function.GetDeclaration(true, true, false, false);
    if (module_name == nullptr || *module_name == '\0' ||
        declaration == nullptr || *declaration == '\0') {
        return false;
    }
    identity = {module_name, declaration};
    return true;
}

struct reflected_static_jit_identity {
    std::string module_name;
    std::string owner_namespace;
    std::string owner_name;
    std::string function_name;
    bool object_bound = false;

    bool operator<(const reflected_static_jit_identity& other) const noexcept {
        return std::tie(
                   module_name, object_bound, owner_namespace, owner_name,
                   function_name) <
            std::tie(
                   other.module_name, other.object_bound, other.owner_namespace,
                   other.owner_name, other.function_name);
    }

    bool operator==(const reflected_static_jit_identity& other) const noexcept {
        return module_name == other.module_name && object_bound == other.object_bound &&
            owner_namespace == other.owner_namespace && owner_name == other.owner_name &&
            function_name == other.function_name;
    }
};

bool static_jit_reflected_identity(
    asIScriptFunction& function,
    reflected_static_jit_identity& identity) {
    const char* const module_name = function.GetModuleName();
    const char* const function_name = function.GetName();
    if (module_name == nullptr || *module_name == '\0' ||
        function_name == nullptr || *function_name == '\0') {
        return false;
    }

    identity = {};
    identity.module_name = module_name;
    identity.function_name = function_name;
    asITypeInfo* const object_type = function.GetObjectType();
    if (object_type == nullptr) {
        const char* const name_space = function.GetNamespace();
        if (name_space == nullptr) return false;
        identity.owner_namespace = name_space;
        return true;
    }

    const char* const owner_name = object_type->GetName();
    const char* const owner_namespace = object_type->GetNamespace();
    if (owner_name == nullptr || *owner_name == '\0' || owner_namespace == nullptr) {
        return false;
    }
    identity.object_bound = true;
    identity.owner_name = owner_name;
    identity.owner_namespace = owner_namespace;
    return true;
}

bool collect_reflected_static_jit_identities(
    const lexical_preprocess_result& source,
    std::vector<reflected_static_jit_identity>& identities,
    std::string& detail) {
    std::unordered_set<std::string> module_names;
    module_names.reserve(source.modules.size());
    for (const lexical_module_description& module : source.modules) {
        if (module.module_name.empty() || module.module_name.find('\0') != std::string::npos ||
            !module_names.insert(module.module_name).second) {
            detail = "StaticJIT UFUNCTION coverage has an invalid or duplicate module";
            return false;
        }
        for (const preprocessed_class_description& type : module.classes) {
            if (type.class_name.empty() || type.class_name.find('\0') != std::string::npos ||
                type.name_space.find('\0') != std::string::npos) {
                detail = "StaticJIT UFUNCTION coverage has an invalid owner identity";
                return false;
            }
            for (const preprocessed_function_description& function : type.methods) {
                if (function.script_function_name.empty() ||
                    function.script_function_name.find('\0') != std::string::npos) {
                    detail = "StaticJIT UFUNCTION coverage has an invalid function identity";
                    return false;
                }
                identities.push_back({
                    module.module_name,
                    type.name_space,
                    type.is_statics_class ? std::string{} : type.class_name,
                    function.script_function_name,
                    !type.is_statics_class});
            }
        }
    }
    std::sort(identities.begin(), identities.end());
    if (std::adjacent_find(identities.begin(), identities.end()) != identities.end()) {
        detail = "StaticJIT UFUNCTION coverage is ambiguous";
        return false;
    }
    return true;
}

template <typename T>
bool sorted_unique(const std::vector<T>& values) {
    return std::is_sorted(values.begin(), values.end()) &&
        std::adjacent_find(values.begin(), values.end()) == values.end();
}

} // namespace

engine_bridge_result derive_shipping_static_jit_module_coverage(
    const std::vector<asIScriptModule*>& base_modules,
    shipping_static_jit_coverage& coverage) {
    try {
        struct function_snapshot {
            asCScriptFunction* function = nullptr;
            asDWORD traits = 0U;
            std::string declaration;
        };
        struct module_snapshot {
            std::string name;
            std::vector<function_snapshot> functions;
        };

        std::vector<module_snapshot> snapshots;
        snapshots.reserve(base_modules.size());
        shipping_static_jit_candidates all_candidates;
        std::vector<std::pair<std::string, std::string>> function_identities;
        std::unordered_set<std::string> names;
        names.reserve(base_modules.size());
        for (asIScriptModule* const module_interface : base_modules) {
            if (module_interface == nullptr || module_interface->GetName() == nullptr) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "StaticJIT coverage received a module without identity",
                    asINVALID_ARG);
            }
            auto& module = static_cast<asCModule&>(*module_interface);
            module_snapshot snapshot;
            snapshot.name = module.GetName();
            if (snapshot.name.empty() || !names.insert(snapshot.name).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "StaticJIT coverage received an empty or duplicate module identity",
                    asINVALID_ARG);
            }
            for (asUINT index = 0U; index < module.scriptFunctions.GetLength(); ++index) {
                asCScriptFunction* const function = module.scriptFunctions[index];
                if (function == nullptr || function->funcType != asFUNC_SCRIPT) continue;
                std::pair<std::string, std::string> identity;
                if (!static_jit_function_identity(*function, identity) ||
                    identity.first != snapshot.name) {
                    return failure(
                        engine_bridge_phase::preflight, kNoModule,
                        "StaticJIT coverage could not derive a stable function identity",
                        asINVALID_CONFIGURATION);
                }
                snapshot.functions.push_back({
                    function, function->traits.traits, std::move(identity.second)});
                function_identities.emplace_back(
                    identity.first, snapshot.functions.back().declaration);
                all_candidates.functions.push_back(function);
            }
            snapshots.push_back(std::move(snapshot));
        }

        std::sort(function_identities.begin(), function_identities.end());
        if (!sorted_unique(function_identities)) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "StaticJIT coverage has an ambiguous function identity",
                asINVALID_CONFIGURATION);
        }

        const engine_bridge_result analyzed = apply_shipping_static_jit_checkpoint(
            base_modules, all_candidates);
        if (!analyzed.succeeded()) return analyzed;

        shipping_static_jit_coverage staged;
        staged.base_module_names.reserve(snapshots.size());
        staged.fully_analyzed_module_names.reserve(snapshots.size());
        for (const module_snapshot& snapshot : snapshots) {
            staged.base_module_names.push_back(snapshot.name);
            const bool changed = std::any_of(
                snapshot.functions.begin(), snapshot.functions.end(),
                [](const function_snapshot& entry) {
                    return entry.function->traits.traits != entry.traits;
                });
            // An empty base module carries no observable evidence that the
            // Shipping stage-3 pass actually covered future source functions.
            // Treat only non-empty fixed points as fully analyzed; otherwise
            // a later edit of an empty placeholder module would inherit FINAL
            // without a matching sealed function identity.
            if (!changed && !snapshot.functions.empty()) {
                staged.fully_analyzed_module_names.push_back(snapshot.name);
                continue;
            }
            for (const function_snapshot& function : snapshot.functions) {
                if ((function.traits & asTRAIT_FINAL) != 0U) {
                    staged.retained_final_functions.emplace_back(
                        snapshot.name, function.declaration);
                }
            }
        }
        std::sort(staged.base_module_names.begin(), staged.base_module_names.end());
        std::sort(
            staged.fully_analyzed_module_names.begin(),
            staged.fully_analyzed_module_names.end());
        std::sort(
            staged.retained_final_functions.begin(),
            staged.retained_final_functions.end());
        coverage = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed while deriving StaticJIT module coverage",
            asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected StaticJIT coverage failure", asERROR);
    }
}

engine_bridge_result apply_shipping_static_jit_coverage_checkpoint(
    const std::vector<asIScriptModule*>& modules,
    const shipping_static_jit_candidates& candidates,
    const shipping_static_jit_coverage& coverage,
    const lexical_preprocess_result& source) {
    try {
        if (!sorted_unique(coverage.base_module_names) ||
            !sorted_unique(coverage.fully_analyzed_module_names) ||
            !sorted_unique(coverage.retained_final_functions) ||
            !std::includes(
                coverage.base_module_names.begin(), coverage.base_module_names.end(),
                coverage.fully_analyzed_module_names.begin(),
                coverage.fully_analyzed_module_names.end())) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "Shipping StaticJIT coverage is not canonical or self-consistent",
                asINVALID_ARG);
        }
        for (const auto& identity : coverage.retained_final_functions) {
            if (!std::binary_search(
                    coverage.base_module_names.begin(),
                    coverage.base_module_names.end(), identity.first)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT retained function has no base module",
                    asINVALID_ARG);
            }
            if (std::binary_search(
                    coverage.fully_analyzed_module_names.begin(),
                    coverage.fully_analyzed_module_names.end(), identity.first)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT retained and fully analyzed coverage overlap",
                    asINVALID_ARG);
            }
        }

        std::vector<reflected_static_jit_identity> reflected_functions;
        std::string reflected_detail;
        if (!collect_reflected_static_jit_identities(
                source, reflected_functions, reflected_detail)) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                std::move(reflected_detail), asINVALID_CONFIGURATION);
        }

        // Preprocessor descriptors carry the exact exported script name but
        // not an overload signature. Prove that each descriptor maps to one
        // and only one current stage-3 candidate before any trait is changed;
        // otherwise a same-name non-UFUNCTION overload could inherit FINAL.
        std::vector<reflected_static_jit_identity> candidate_reflected_identities;
        candidate_reflected_identities.reserve(candidates.functions.size());
        for (asIScriptFunction* const candidate : candidates.functions) {
            if (candidate == nullptr || candidate->GetFuncType() != asFUNC_SCRIPT) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT coverage received an invalid candidate",
                    asINVALID_ARG);
            }
            reflected_static_jit_identity reflected_identity;
            if (!static_jit_reflected_identity(*candidate, reflected_identity)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidate has no reflected identity",
                    asINVALID_CONFIGURATION);
            }
            candidate_reflected_identities.push_back(std::move(reflected_identity));
        }
        std::sort(
            candidate_reflected_identities.begin(),
            candidate_reflected_identities.end());
        for (const reflected_static_jit_identity& reflected : reflected_functions) {
            const auto matches = std::equal_range(
                candidate_reflected_identities.begin(),
                candidate_reflected_identities.end(), reflected);
            if (std::distance(matches.first, matches.second) != 1) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT UFUNCTION identity does not map uniquely",
                    asINVALID_CONFIGURATION);
            }
        }

        shipping_static_jit_candidates projected;
        projected.functions.reserve(candidates.functions.size());
        for (std::size_t candidate_index = 0U;
             candidate_index < candidates.functions.size(); ++candidate_index) {
            asIScriptFunction* const candidate = candidates.functions[candidate_index];
            std::pair<std::string, std::string> identity;
            if (!static_jit_function_identity(*candidate, identity)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidate has no stable function identity",
                    asINVALID_CONFIGURATION);
            }
            const bool base_module = std::binary_search(
                coverage.base_module_names.begin(),
                coverage.base_module_names.end(), identity.first);
            const bool fully_analyzed = std::binary_search(
                coverage.fully_analyzed_module_names.begin(),
                coverage.fully_analyzed_module_names.end(), identity.first);
            const bool retained_final = std::binary_search(
                coverage.retained_final_functions.begin(),
                coverage.retained_final_functions.end(), identity);
            auto& concrete = static_cast<asCScriptFunction&>(*candidate);
            constexpr asDWORD retained_role_mask =
                asTRAIT_CONSTRUCTOR | asTRAIT_DESTRUCTOR | asTRAIT_GENERATED_FUNCTION;
            const bool retained_role =
                (concrete.traits.traits & retained_role_mask) != 0U;
            reflected_static_jit_identity reflected_identity;
            const bool reflected_identity_valid =
                static_jit_reflected_identity(*candidate, reflected_identity);
            if (!reflected_identity_valid) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "Shipping StaticJIT candidate has no reflected identity",
                    asINVALID_CONFIGURATION);
            }
            const bool reflected = std::binary_search(
                reflected_functions.begin(), reflected_functions.end(),
                reflected_identity);
            if (!base_module || fully_analyzed ||
                (retained_final && (retained_role || reflected))) {
                projected.functions.push_back(candidate);
            }
        }
        return apply_shipping_static_jit_checkpoint(modules, projected);
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed while projecting StaticJIT coverage",
            asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected StaticJIT coverage projection failure", asERROR);
    }
}

static engine_bridge_result export_module_in_place(
    asIScriptModule& module_interface,
    const map_string& module_key,
    const archive_string& script_relative_filename,
    const std::int64_t code_hash,
    precompiled_module& output,
    reference_exporter* const exporter_pointer) {
    try {
        auto& module = static_cast<asCModule&>(module_interface);
        std::string key_name;
        if (!plain_module_name(module_key, key_name) || key_name != module.GetName()) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "module key must exactly match the engine module name");
        }
        precompiled_module encoded;
        encoded.module_name.bytes = module.GetName();
        encoded.code_hash = code_hash;
        encoded.script_relative_filename = script_relative_filename;
        for (asUINT index = 0U; index < module.importedModules.GetLength(); ++index) {
            if (module.importedModules[index] != nullptr) {
                encoded.imported_modules.push_back(
                    archive_string{module.importedModules[index]->GetName()});
            }
        }
        for (asUINT index = 0U; index < module.bindInformations.GetLength(); ++index) {
            sBindInfo* binding = module.bindInformations[index];
            if (binding == nullptr || binding->importedFunctionSignature == nullptr) {
                return failure(
                    engine_bridge_phase::export_module, kNoModule,
                    "module contains an incomplete imported-function binding", asERROR);
            }
            if (exporter_pointer == nullptr) {
                return failure(
                    engine_bridge_phase::export_module, kNoModule,
                    "function-import export requires cache reference tables");
            }
            asCScriptFunction& function = *binding->importedFunctionSignature;
            function_import imported;
            imported.imported_from_module.bytes = binding->importFromModule.AddressOf();
            imported.signature.name.bytes = function.name.AddressOf();
            imported.signature.name_space.bytes = function.GetNamespace();
            std::string detail;
            if (!exporter_pointer->export_data_type(
                    function.returnType, imported.signature.return_type, detail)) {
                return failure(engine_bridge_phase::export_module, kNoModule, std::move(detail));
            }
            imported.signature.parameter_types.resize(function.parameterTypes.GetLength());
            imported.signature.parameter_flags.resize(function.parameterTypes.GetLength());
            imported.signature.parameter_default_args.resize(function.parameterTypes.GetLength());
            for (asUINT parameter = 0U;
                 parameter < function.parameterTypes.GetLength(); ++parameter) {
                if (!exporter_pointer->export_data_type(
                        function.parameterTypes[parameter],
                        imported.signature.parameter_types[parameter], detail)) {
                    return failure(
                        engine_bridge_phase::export_module, kNoModule, std::move(detail));
                }
                imported.signature.parameter_flags[parameter] =
                    static_cast<std::int32_t>(function.inOutFlags[parameter]);
                if (function.defaultArgs[parameter] != nullptr) {
                    imported.signature.parameter_default_args[parameter].bytes =
                        function.defaultArgs[parameter]->AddressOf();
                }
            }
            encoded.function_imports.push_back(std::move(imported));
        }
        if (module.classTypes.GetLength() != 0U && exporter_pointer == nullptr) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "class export requires cache reference tables");
        }
        for (asUINT index = 0U; index < module.classTypes.GetLength(); ++index) {
            precompiled_class type_record;
            engine_bridge_result result = export_class(
                *module.engine, *module.classTypes[index], *exporter_pointer, type_record);
            if (!result.succeeded()) {
                return result;
            }
            encoded.classes.push_back(std::move(type_record));
        }
        for (asUINT index = 0U; index < module.scriptFunctions.GetLength(); ++index) {
            asCScriptFunction* function = module.scriptFunctions[index];
            if (function->objectType != nullptr) {
                continue;
            }
            precompiled_function function_record;
            engine_bridge_result result =
                export_function(*function, exporter_pointer, function_record);
            if (!result.succeeded()) {
                return result;
            }
            encoded.functions.push_back(std::move(function_record));
        }
        for (asUINT index = 0U; index < module.enumTypes.GetLength(); ++index) {
            const asCEnumType& type = *module.enumTypes[index];
            precompiled_enum enumeration;
            enumeration.name.bytes = type.name.AddressOf();
            enumeration.name_space.bytes = type.GetNamespace();
            for (asUINT value_index = 0U; value_index < type.enumValues.GetLength(); ++value_index) {
                enumeration.names.push_back(
                    archive_string{type.enumValues[value_index]->name.AddressOf()});
                enumeration.values.push_back(type.enumValues[value_index]->value);
            }
            encoded.enums.push_back(std::move(enumeration));
        }

        bool global_failed = false;
        std::string global_failure;
        module.scriptGlobals.IterateAll([&](asCGlobalProperty* property) {
            if (global_failed) {
                return;
            }
            precompiled_global global;
            global.name.bytes = property->name.AddressOf();
            global.name_space.bytes = property->nameSpace->name.AddressOf();
            if (exporter_pointer != nullptr) {
                if (!exporter_pointer->export_data_type(
                        property->type, global.type, global_failure)) {
                    global_failed = true;
                    return;
                }
            } else {
                global.type.is_reference = property->type.IsReference();
                global.type.is_object_const = property->type.IsObjectConst();
                global.type.is_const_handle = property->type.IsReadOnly();
                global.type.token_type =
                    static_cast<std::int32_t>(property->type.GetTokenType());
                if (property->type.GetTypeInfo() != nullptr) {
                    global_failed = true;
                    global_failure = "object global requires a cache reference exporter";
                    return;
                }
            }
            global.is_pure_constant = property->isPureConstant;
            global.pure_constant_value = property->storage;
            global.is_default_init = property->isDefaultInit;
            if (!global.is_pure_constant && !global.is_default_init &&
                property->GetInitFunc() != nullptr) {
                global.has_init_function = true;
                const engine_bridge_result init_result = export_function(
                    *property->GetInitFunc(), exporter_pointer, global.init_function);
                if (!init_result.succeeded()) {
                    global_failed = true;
                    global_failure = init_result.detail;
                    return;
                }
            }
            encoded.global_variables.push_back(std::move(global));
        });
        if (global_failed) {
            return failure(
                engine_bridge_phase::export_module, kNoModule, std::move(global_failure));
        }
        output = std::move(encoded);
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "allocation failed while exporting precompiled module", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::export_module, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "unexpected precompiled module export failure", asERROR);
    }
}

engine_bridge_result export_module_checkpoint(
    asIScriptModule& module_interface,
    const map_string& module_key,
    const archive_string& script_relative_filename,
    const std::int64_t code_hash,
    precompiled_module& output,
    cache* reference_tables) {
    try {
        auto& module = static_cast<asCModule&>(module_interface);
        cache staged_references;
        std::unique_ptr<reference_exporter> exporter;
        if (reference_tables != nullptr) {
            staged_references = *reference_tables;
            exporter = std::make_unique<reference_exporter>(
                *module.engine, staged_references);
        }
        precompiled_module staged_output;
        engine_bridge_result result = export_module_in_place(
            module_interface, module_key, script_relative_filename, code_hash,
            staged_output, exporter.get());
        if (!result.succeeded()) {
            return result;
        }
        output = std::move(staged_output);
        if (reference_tables != nullptr) {
            *reference_tables = std::move(staged_references);
        }
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "allocation failed while staging precompiled module export",
            asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::export_module, kNoModule,
            "unexpected staged precompiled module export failure", asERROR);
    }
}

static bool target_archive_ansi_from_utf8(
    const std::string& utf8,
    std::string& output);

engine_bridge_result export_mixed_graph_checkpoint(
    const cache& base,
    const lexical_preprocess_result& source,
    const std::vector<asIScriptModule*>& modules,
    const std::array<std::uint8_t, 16U>& data_guid,
    const std::int32_t build_identifier,
    cache& output,
    registry_runtime& registry,
    const bool mark_non_uproperty_properties_as_transient) {
    try {
        if (!source.ok || build_identifier == -1) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "mixed export requires successful preprocessing and a qualified build identifier",
                asINVALID_ARG);
        }
        std::unordered_map<std::string, std::size_t> base_by_name;
        base_by_name.reserve(base.modules.size());
        for (std::size_t index = 0U; index < base.modules.size(); ++index) {
            const std::string& name = base.modules[index].second.module_name.bytes;
            if (name.empty() || !base_by_name.emplace(name, index).second) {
                return failure(
                    engine_bridge_phase::preflight, index,
                    "base cache has an empty or duplicate module identity",
                    asINVALID_ARG);
            }
        }
        std::unordered_map<std::string, const lexical_module_description*> source_by_name;
        source_by_name.reserve(source.modules.size());
        std::vector<const lexical_module_description*> additions;
        additions.reserve(source.modules.size());
        for (const lexical_module_description& description : source.modules) {
            if (description.module_name.empty() ||
                !source_by_name.emplace(description.module_name, &description).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "source descriptors have an empty or duplicate module identity",
                    asINVALID_ARG);
            }
            if (base_by_name.find(description.module_name) == base_by_name.end()) {
                additions.push_back(&description);
            }
        }
        const std::size_t expected_count = base.modules.size() + additions.size();
        if (modules.size() != expected_count) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "final engine module count does not match the mixed graph",
                asINVALID_ARG);
        }
        std::vector<std::string> expected_names;
        expected_names.reserve(expected_count);
        for (const auto& entry : base.modules) {
            expected_names.push_back(entry.second.module_name.bytes);
        }
        for (const lexical_module_description* const addition : additions) {
            expected_names.push_back(addition->module_name);
        }
        for (std::size_t index = 0U; index < modules.size(); ++index) {
            if (modules[index] == nullptr ||
                expected_names[index] != modules[index]->GetName()) {
                return failure(
                    engine_bridge_phase::preflight, index,
                    "final engine module order/name does not match the mixed graph",
                    asINVALID_ARG);
            }
        }

        cache staged;
        staged.data_guid = data_guid;
        staged.build_identifier = build_identifier;
        if (source.modules.empty() && source.static_names.empty()) {
            staged.static_names = base.static_names;
        } else {
            if (source.static_names.size() < base.static_names.size()) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "preprocessor static-name output lost the base prefix",
                    asINVALID_ARG);
            }
            for (std::size_t index = 0U; index < base.static_names.size(); ++index) {
                if (source.static_names[index] != base.static_names[index].bytes) {
                    return failure(
                        engine_bridge_phase::preflight, kNoModule,
                    "preprocessor static-name prefix differs from the base cache",
                        asINVALID_ARG);
                }
            }
            staged.static_names = base.static_names;
            staged.static_names.reserve(source.static_names.size());
            for (std::size_t index = base.static_names.size();
                 index < source.static_names.size(); ++index) {
                archive_string archived;
                if (!target_archive_ansi_from_utf8(
                        source.static_names[index], archived.bytes)) {
                    return failure(
                        engine_bridge_phase::preflight, kNoModule,
                        "source graph contains a non-canonical UTF-8 static name",
                        asINVALID_ARG);
                }
                staged.static_names.push_back(std::move(archived));
            }
        }

        staged.modules.reserve(modules.size());
        std::unique_ptr<reference_exporter> graph_exporter;
        if (!modules.empty()) {
            auto& first_module = static_cast<asCModule&>(*modules.front());
            graph_exporter = std::make_unique<reference_exporter>(
                *first_module.engine, staged);
        }
        for (std::size_t index = 0U; index < modules.size(); ++index) {
            const std::string& name = expected_names[index];
            const auto source_record = source_by_name.find(name);
            const auto cached_record = base_by_name.find(name);
            map_string key;
            archive_string script_path;
            std::int64_t code_hash = 0;
            if (cached_record != base_by_name.end()) {
                key = base.modules[cached_record->second].first;
            } else {
                key.payload.assign(name.begin(), name.end());
            }
            if (source_record != source_by_name.end()) {
                code_hash = source_record->second->code_hash;
                if (!source_record->second->code.empty()) {
                    script_path.bytes =
                        source_record->second->code.front().relative_path;
                }
            } else {
                const precompiled_module& cached =
                    base.modules[cached_record->second].second;
                code_hash = cached.code_hash;
                script_path = cached.script_relative_filename;
            }

            precompiled_module rebuilt;
            engine_bridge_result result = export_module_in_place(
                *modules[index], key, script_path, code_hash, rebuilt,
                graph_exporter.get());
            if (!result.succeeded()) {
                result.module_index = index;
                return result;
            }
            metadata_projection_result metadata;
            if (source_record != source_by_name.end()) {
                class_generator_capability_table capabilities;
                engine_bridge_result capability_result =
                    collect_class_generator_capabilities(
                        static_cast<asCModule&>(*modules[index]), registry, capabilities);
                if (!capability_result.succeeded()) {
                    capability_result.module_index = index;
                    return capability_result;
                }
                metadata = project_preprocessed_metadata(
                    *source_record->second, rebuilt,
                    mark_non_uproperty_properties_as_transient, &capabilities);
            } else {
                metadata = preserve_cached_metadata(
                    base.modules[cached_record->second].second, rebuilt);
            }
            if (!metadata.ok) {
                engine_bridge_result metadata_failure = failure(
                    engine_bridge_phase::export_module, index,
                    std::move(metadata.detail), asERROR);
                metadata_failure.is_compile_diagnostic = metadata.is_compile_diagnostic;
                metadata_failure.diagnostic_source = std::move(metadata.diagnostic_source);
                metadata_failure.diagnostic_line = metadata.diagnostic_line;
                metadata_failure.diagnostic_column = metadata.diagnostic_column;
                return metadata_failure;
            }
            staged.modules.emplace_back(std::move(key), std::move(rebuilt));
        }

        codec_error codec;
        std::vector<std::uint8_t> encoded;
        if (!encode(staged, encoded, codec)) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "mixed cache failed final wire validation at " + codec.field +
                    ": " + codec.detail,
                asERROR);
        }
        output = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed while exporting mixed graph", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected mixed graph export failure", asERROR);
    }
}

template <typename Visitor>
static void visit_module_functions(precompiled_module& module, Visitor&& visitor) {
    for (precompiled_function& function : module.functions) visitor(function);
    for (precompiled_class& type : module.classes) {
        for (precompiled_function& function : type.methods) visitor(function);
        for (precompiled_function& function : type.constructors) visitor(function);
        for (precompiled_function& function : type.behaviour_functions) visitor(function);
    }
    for (precompiled_global& global : module.global_variables) {
        if (global.has_init_function) visitor(global.init_function);
    }
}

static engine_bridge_result rebase_projected_static_names(
    cache& projected,
    const std::unordered_map<std::size_t, std::size_t>& global_to_local) {
    std::unordered_set<std::int64_t> static_name_functions;
    for (const auto& entry : projected.function_references) {
        if (entry.second.name.bytes == "__STATIC_NAME") {
            static_name_functions.insert(entry.first);
        }
    }
    bool failed = false;
    std::string detail;
    for (auto& module_entry : projected.modules) {
        visit_module_functions(module_entry.second, [&](precompiled_function& function) {
            if (failed) return;
            std::size_t offset = 0U;
            while (offset < function.byte_code.size()) {
                const auto opcode =
                    static_cast<asEBCInstr>(function.byte_code[offset] & 0xff);
                const std::size_t size =
                    static_cast<std::size_t>(asBCTypeSize[asBCInfo[opcode].type]);
                if (size == 0U || offset > function.byte_code.size() - size) {
                    failed = true;
                    detail = "projected function bytecode is not instruction aligned";
                    return;
                }
                const std::size_t next = offset + size;
                if (opcode == asBC_PshC4 && size >= 2U && next < function.byte_code.size()) {
                    const auto next_opcode =
                        static_cast<asEBCInstr>(function.byte_code[next] & 0xff);
                    const std::size_t next_size =
                        static_cast<std::size_t>(asBCTypeSize[asBCInfo[next_opcode].type]);
                    if (next_opcode == asBC_CALLSYS && next_size >= 3U &&
                        next <= function.byte_code.size() - next_size) {
                        const std::uint64_t low =
                            static_cast<std::uint32_t>(function.byte_code[next + 1U]);
                        const std::uint64_t high =
                            static_cast<std::uint32_t>(function.byte_code[next + 2U]);
                        const auto function_key = static_cast<std::int64_t>(low | (high << 32U));
                        if (static_name_functions.find(function_key) !=
                            static_name_functions.end()) {
                            const std::int64_t source_index = function.byte_code[offset + 1U];
                            if (source_index < 0) {
                                failed = true;
                                detail = "qualification source referenced a static name outside its projected rows";
                                return;
                            }
                            const auto local = global_to_local.find(
                                static_cast<std::size_t>(source_index));
                            if (local == global_to_local.end() ||
                                local->second > static_cast<std::size_t>(
                                    (std::numeric_limits<std::int32_t>::max)())) {
                                failed = true;
                                detail = "qualification source referenced a static name outside its projected rows";
                                return;
                            }
                            function.byte_code[offset + 1U] = static_cast<std::int32_t>(
                                local->second);
                        }
                    }
                }
                offset = next;
            }
        });
        if (failed) break;
    }
    return failed
        ? failure(engine_bridge_phase::export_module, kNoModule, std::move(detail), asERROR)
        : engine_bridge_result{};
}

static bool qualification_rebased_code_hash(
    asIScriptEngine& engine,
    const lexical_module_description& description,
    const std::unordered_map<std::size_t, std::size_t>& global_to_local,
    std::int64_t& output,
    std::string& detail) {
    struct token_span {
        std::size_t start = 0U;
        std::size_t length = 0U;
        asETokenClass kind = asTC_UNKNOWN;
    };
    struct replacement {
        std::size_t start = 0U;
        std::size_t length = 0U;
        std::string text;
    };
    output = 0;
    for (const preprocessed_code_section& section : description.code) {
        std::string code = section.conditioned_code;
        std::vector<replacement> replacements;
        for (std::size_t position = 0U; position < code.size();) {
            asUINT token_length = 0U;
            const asETokenClass token_class = engine.ParseToken(
                code.data() + position, code.size() - position, &token_length);
            if (token_length == 0U) {
                detail = "qualification source tokenizer made no progress";
                return false;
            }
            if (token_class != asTC_IDENTIFIER ||
                code.compare(position, token_length, "__STATIC_NAME") != 0) {
                position += token_length;
                continue;
            }

            std::size_t cursor = position + token_length;
            const auto next_significant = [&](token_span& span) -> bool {
                while (cursor < code.size()) {
                    asUINT length = 0U;
                    const asETokenClass kind = engine.ParseToken(
                        code.data() + cursor, code.size() - cursor, &length);
                    if (length == 0U) return false;
                    span = {cursor, static_cast<std::size_t>(length), kind};
                    cursor += length;
                    if (kind != asTC_WHITESPACE && kind != asTC_COMMENT) return true;
                }
                return false;
            };
            token_span open;
            token_span digits;
            token_span close;
            if (!next_significant(open) ||
                code.compare(open.start, open.length, "(") != 0 ||
                !next_significant(digits) || !next_significant(close) ||
                code.compare(close.start, close.length, ")") != 0) {
                position += token_length;
                continue;
            }
            std::size_t source_index = 0U;
            for (std::size_t index = digits.start;
                 index < digits.start + digits.length; ++index) {
                if (code[index] < '0' || code[index] > '9') {
                    detail = "qualification source contains a malformed static-name index";
                    return false;
                }
                const std::size_t digit = static_cast<std::size_t>(code[index] - '0');
                if (source_index >
                    ((std::numeric_limits<std::size_t>::max)() - digit) / 10U) {
                    detail = "qualification source contains an overflowing static-name index";
                    return false;
                }
                source_index = source_index * 10U + digit;
            }
            const auto local = global_to_local.find(source_index);
            if (local == global_to_local.end()) {
                detail = "qualification source referenced a static name outside its projected rows";
                return false;
            }
            replacements.push_back({
                digits.start, digits.length, std::to_string(local->second)});
            position = close.start + close.length;
        }
        for (auto replacement = replacements.rbegin();
             replacement != replacements.rend(); ++replacement) {
            code.replace(replacement->start, replacement->length, replacement->text);
        }
        std::int64_t section_hash = 0;
        if (!compute_processed_code_hash_utf8(code, section_hash)) {
            detail = "qualification source is not canonical UTF-8 after static-name rebasing";
            return false;
        }
        output ^= section_hash;
    }
    return true;
}

static bool target_archive_ansi_from_utf8(
    const std::string& utf8,
    std::string& output) {
    output.clear();
    output.reserve(utf8.size());
    for (std::size_t index = 0U; index < utf8.size();) {
        const auto first = static_cast<std::uint8_t>(utf8[index++]);
        if (first < 0x80U) {
            output.push_back(static_cast<char>(first));
            continue;
        }
        std::size_t continuation_count = 0U;
        std::uint32_t scalar = 0U;
        if (first >= 0xc2U && first <= 0xdfU) {
            continuation_count = 1U;
            scalar = first & 0x1fU;
        } else if (first >= 0xe0U && first <= 0xefU) {
            continuation_count = 2U;
            scalar = first & 0x0fU;
        } else if (first >= 0xf0U && first <= 0xf4U) {
            continuation_count = 3U;
            scalar = first & 0x07U;
        } else {
            return false;
        }
        if (continuation_count > utf8.size() - index) return false;
        for (std::size_t part = 0U; part < continuation_count; ++part) {
            const auto value = static_cast<std::uint8_t>(utf8[index++]);
            if ((value & 0xc0U) != 0x80U) return false;
            scalar = (scalar << 6U) | (value & 0x3fU);
        }
        if ((continuation_count == 2U && scalar < 0x800U) ||
            (continuation_count == 3U && scalar < 0x10000U) ||
            scalar > 0x10ffffU || (scalar >= 0xd800U && scalar <= 0xdfffU)) {
            return false;
        }
        // FStringInArchive assigns FString through TCHAR_TO_ANSI. The pinned
        // target substitutes every non-ANSI UTF-16 code unit with one '?'. A
        // scalar outside the BMP is therefore archived as the two '?' from its
        // surrogate pair.
        output.push_back('?');
        if (scalar > 0xffffU) output.push_back('?');
    }
    return true;
}

engine_bridge_result export_source_graph_checkpoint(
    const cache& base,
    const lexical_preprocess_result& source,
    const std::vector<asIScriptModule*>& modules,
    const std::array<std::uint8_t, 16U>& data_guid,
    const std::int32_t build_identifier,
    cache& output,
    registry_runtime& registry,
    const bool mark_non_uproperty_properties_as_transient) {
    try {
        if (!source.ok || source.modules.empty() || modules.empty() || build_identifier == -1 ||
            source.static_names.size() < base.static_names.size()) {
            return failure(
                engine_bridge_phase::preflight, kNoModule,
                "source projection requires successful preprocessing and a qualified base prefix",
                asINVALID_ARG);
        }
        for (std::size_t index = 0U; index < base.static_names.size(); ++index) {
            if (source.static_names[index] != base.static_names[index].bytes) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "source projection static-name prefix differs from the sealed base",
                    asINVALID_ARG);
            }
        }
        std::unordered_map<std::string, asIScriptModule*> module_by_name;
        module_by_name.reserve(modules.size());
        for (asIScriptModule* const module : modules) {
            if (module == nullptr || !module_by_name.emplace(module->GetName(), module).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "source projection received null or duplicate engine modules",
                    asINVALID_ARG);
            }
        }

        cache staged;
        staged.data_guid = data_guid;
        staged.build_identifier = build_identifier;
        std::unordered_map<std::size_t, std::size_t> global_to_local;
        global_to_local.reserve(source.static_name_uses.size());
        staged.static_names.reserve(source.static_name_uses.size());
        for (const static_name_use& use : source.static_name_uses) {
            if (use.global_index >= source.static_names.size() || use.spelling.empty() ||
                !global_to_local.emplace(
                    use.global_index, staged.static_names.size()).second) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "source projection has an invalid or repeated static-name use",
                    asINVALID_ARG);
            }
            archive_string archived;
            if (!target_archive_ansi_from_utf8(
                    use.spelling, archived.bytes)) {
                return failure(
                    engine_bridge_phase::preflight, kNoModule,
                    "source projection contains a non-canonical UTF-8 static name",
                    asINVALID_ARG);
            }
            staged.static_names.push_back(std::move(archived));
        }
        auto& first_module = static_cast<asCModule&>(*modules.front());
        reference_exporter exporter(*first_module.engine, staged);
        staged.modules.reserve(source.modules.size());
        for (std::size_t index = 0U; index < source.modules.size(); ++index) {
            const lexical_module_description& description = source.modules[index];
            const auto found = module_by_name.find(description.module_name);
            if (found == module_by_name.end()) {
                return failure(
                    engine_bridge_phase::export_module, index,
                    "source projection could not find its compiled engine module", asERROR);
            }
            map_string key;
            key.payload.assign(description.module_name.begin(), description.module_name.end());
            archive_string script_path;
            if (!description.code.empty()) {
                script_path.bytes = description.code.front().relative_path;
            }
            std::int64_t projected_code_hash = 0;
            std::string code_hash_detail;
            if (!qualification_rebased_code_hash(
                    *first_module.engine, description, global_to_local,
                    projected_code_hash, code_hash_detail)) {
                return failure(
                    engine_bridge_phase::export_module, index,
                    std::move(code_hash_detail), asERROR);
            }
            precompiled_module rebuilt;
            engine_bridge_result result = export_module_in_place(
                *found->second, key, script_path, projected_code_hash, rebuilt, &exporter);
            if (!result.succeeded()) {
                result.module_index = index;
                return result;
            }
            class_generator_capability_table capabilities;
            engine_bridge_result capability_result = collect_class_generator_capabilities(
                static_cast<asCModule&>(*found->second), registry, capabilities);
            if (!capability_result.succeeded()) {
                capability_result.module_index = index;
                return capability_result;
            }
            metadata_projection_result metadata = project_preprocessed_metadata(
                description, rebuilt, mark_non_uproperty_properties_as_transient, &capabilities);
            if (!metadata.ok) {
                engine_bridge_result metadata_failure = failure(
                    engine_bridge_phase::export_module, index,
                    std::move(metadata.detail), asERROR);
                metadata_failure.is_compile_diagnostic = metadata.is_compile_diagnostic;
                metadata_failure.diagnostic_source = std::move(metadata.diagnostic_source);
                metadata_failure.diagnostic_line = metadata.diagnostic_line;
                metadata_failure.diagnostic_column = metadata.diagnostic_column;
                return metadata_failure;
            }
            // Metadata projection faithfully carries the product graph's
            // global hash. Qualification uses the independent local FName
            // table, so restore the matching locally rebased hash here.
            rebuilt.code_hash = projected_code_hash;
            staged.modules.emplace_back(std::move(key), std::move(rebuilt));
        }
        engine_bridge_result rebase =
            rebase_projected_static_names(staged, global_to_local);
        if (!rebase.succeeded()) return rebase;
        codec_error codec;
        std::vector<std::uint8_t> encoded;
        if (!encode(staged, encoded, codec)) {
            return failure(
                engine_bridge_phase::export_module, kNoModule,
                "source projection failed final wire validation at " + codec.field +
                    ": " + codec.detail,
                asERROR);
        }
        output = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "allocation failed while exporting the source projection", asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(engine_bridge_phase::cleanup, kNoModule, exception.what(), asERROR);
    } catch (...) {
        return failure(
            engine_bridge_phase::cleanup, kNoModule,
            "unexpected source projection export failure", asERROR);
    }
}

} // namespace gore::as::standalone::precompiled
