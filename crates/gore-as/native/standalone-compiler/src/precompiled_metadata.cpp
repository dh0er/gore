#include "gore_as_standalone/precompiled_metadata.hpp"

#include <algorithm>
#include <exception>
#include <new>
#include <unordered_set>
#include <utility>

namespace gore::as::standalone::precompiled {
namespace {

metadata_projection_result fail(std::string detail) {
    return {false, std::move(detail)};
}

bool metadata_key(
    const preprocessor_metadata& entry,
    std::unordered_set<std::string>& keys) {
    return entry.subject_index == -1 && !entry.name.empty() &&
           entry.name.find('\0') == std::string::npos &&
           entry.value.find('\0') == std::string::npos &&
           keys.emplace(entry.name).second;
}

bool copy_metadata(
    const std::vector<preprocessor_metadata>& input,
    std::vector<archive_string>& specifiers,
    std::vector<archive_string>& values) {
    std::unordered_set<std::string> keys;
    keys.reserve(input.size());
    specifiers.clear();
    values.clear();
    specifiers.reserve(input.size());
    values.reserve(input.size());
    for (const preprocessor_metadata& entry : input) {
        if (!metadata_key(entry, keys)) return false;
        specifiers.push_back({entry.name});
        values.push_back({entry.value});
    }
    return true;
}

precompiled_class* find_class(
    precompiled_module& module,
    const preprocessed_class_description& description) {
    precompiled_class* found = nullptr;
    for (precompiled_class& type : module.classes) {
        if (type.class_name.bytes != description.class_name ||
            type.name_space.bytes != description.name_space) {
            continue;
        }
        if (found != nullptr) return nullptr;
        found = &type;
    }
    return found;
}

precompiled_property* find_property(
    precompiled_class& type,
    const std::string& name) {
    precompiled_property* found = nullptr;
    for (precompiled_property& property : type.properties) {
        if (property.name.bytes != name) continue;
        if (found != nullptr) return nullptr;
        found = &property;
    }
    return found;
}

precompiled_function* find_function(
    std::vector<precompiled_function>& functions,
    const std::string& name) {
    precompiled_function* found = nullptr;
    for (precompiled_function& function : functions) {
        if (function.function_name.bytes != name) continue;
        if (found != nullptr) return nullptr;
        found = &function;
    }
    return found;
}

bool apply_function(
    const preprocessed_function_description& description,
    precompiled_function& function) {
    function.is_unreal_function = true;
    function.unreal_function_name.bytes = description.function_name;
    if (!copy_metadata(
            description.metadata,
            function.metadata_specifiers,
            function.metadata_values)) {
        return false;
    }
    function.blueprint_callable = description.blueprint_callable;
    function.blueprint_override = description.blueprint_override;
    function.blueprint_event = description.blueprint_event;
    function.blueprint_pure = description.blueprint_pure;
    function.net_function = description.net_function;
    function.net_multicast = description.net_multicast;
    function.net_client = description.net_client;
    function.net_server = description.net_server;
    function.net_validate = description.net_validate;
    function.unreliable = description.unreliable;
    function.blueprint_authority_only = description.blueprint_authority_only;
    function.exec = description.exec;
    function.can_override_event = description.can_override_event;
    function.dev_function = description.dev_function;
    function.is_static = description.is_static;
    function.thread_safe = description.thread_safe;
    return true;
}

bool apply_property(
    const preprocessed_property_description& description,
    precompiled_property& property) {
    property.is_unreal_property = true;
    if (!copy_metadata(
            description.metadata,
            property.metadata_specifiers,
            property.metadata_values)) {
        return false;
    }
    property.blueprint_readable = description.blueprint_readable;
    property.blueprint_writable = description.blueprint_writable;
    property.edit_const = description.edit_const;
    property.editable_on_defaults = description.editable_on_defaults;
    property.editable_on_instance = description.editable_on_instance;
    property.instanced_reference = description.instanced_reference;
    property.persistent_instance = description.persistent_instance;
    property.advanced_display = description.advanced_display;
    property.transient = description.transient;
    property.replicated = description.replicated;
    property.replication_condition = description.replication_condition;
    property.skip_replication = description.skip_replication;
    property.skip_serialization = description.skip_serialization;
    property.save_game = description.save_game;
    property.rep_notify = description.rep_notify;
    property.config = description.config;
    property.interp = description.interp;
    property.asset_registry_searchable = description.asset_registry_searchable;
    return true;
}

void copy_function_metadata(
    const precompiled_function& source,
    precompiled_function& target) {
    target.is_unreal_function = source.is_unreal_function;
    target.unreal_function_name = source.unreal_function_name;
    target.metadata_specifiers = source.metadata_specifiers;
    target.metadata_values = source.metadata_values;
    target.blueprint_callable = source.blueprint_callable;
    target.blueprint_override = source.blueprint_override;
    target.blueprint_event = source.blueprint_event;
    target.blueprint_pure = source.blueprint_pure;
    target.net_function = source.net_function;
    target.net_multicast = source.net_multicast;
    target.net_client = source.net_client;
    target.net_server = source.net_server;
    target.net_validate = source.net_validate;
    target.unreliable = source.unreliable;
    target.blueprint_authority_only = source.blueprint_authority_only;
    target.exec = source.exec;
    target.can_override_event = source.can_override_event;
    target.dev_function = source.dev_function;
    target.is_static = source.is_static;
    target.is_const_method = source.is_const_method;
    target.thread_safe = source.thread_safe;
    target.is_no_op = source.is_no_op;
}

bool preserve_function_vector(
    const std::vector<precompiled_function>& source,
    std::vector<precompiled_function>& target) {
    if (source.size() != target.size()) return false;
    for (std::size_t index = 0U; index < source.size(); ++index) {
        if (source[index].function_name.bytes != target[index].function_name.bytes ||
            source[index].name_space.bytes != target[index].name_space.bytes ||
            source[index].parameter_types.size() != target[index].parameter_types.size()) {
            return false;
        }
        copy_function_metadata(source[index], target[index]);
    }
    return true;
}

} // namespace

metadata_projection_result project_preprocessed_metadata(
    const lexical_module_description& description,
    precompiled_module& module) {
    try {
        if (description.module_name.empty() ||
            description.module_name != module.module_name.bytes) {
            return fail("module descriptor does not match the exported module name");
        }
        precompiled_module staged = module;
        staged.code_hash = description.code_hash;
        staged.imported_modules.clear();
        staged.imported_modules.reserve(description.imported_modules.size());
        for (const std::string& imported : description.imported_modules) {
            if (imported.empty() || imported.find('\0') != std::string::npos) {
                return fail("module descriptor contains an invalid direct import");
            }
            staged.imported_modules.push_back({imported});
        }
        staged.post_init_functions.clear();
        staged.post_init_functions.reserve(description.post_init_functions.size());
        for (const std::string& function : description.post_init_functions) {
            if (function.empty() || function.find('\0') != std::string::npos) {
                return fail("module descriptor contains an invalid post-init function");
            }
            staged.post_init_functions.push_back({function});
        }
        staged.statics_class_name.bytes = description.statics_class_name;
        staged.declared_events.clear();
        staged.declared_delegates.clear();
        for (const preprocessed_delegate_description& delegate : description.delegates) {
            if (delegate.delegate_name.empty() ||
                delegate.delegate_name.find('\0') != std::string::npos) {
                return fail("module descriptor contains an invalid delegate name");
            }
            (delegate.multicast ? staged.declared_events : staged.declared_delegates)
                .push_back({delegate.delegate_name});
        }
        if (!description.code.empty()) {
            staged.script_relative_filename.bytes = description.code.front().relative_path;
        }

        const preprocessed_class_description* statics = nullptr;
        for (const preprocessed_class_description& class_description :
             description.classes) {
            if (class_description.is_statics_class) {
                if (statics != nullptr) {
                    return fail("module descriptor contains multiple statics classes");
                }
                statics = &class_description;
                continue;
            }
            precompiled_class* const type = find_class(staged, class_description);
            if (type == nullptr) {
                return fail("reflected class descriptor does not map uniquely to output");
            }
            type->is_in_preprocessor = true;
            type->super_class.bytes = class_description.super_class;
            type->code_super_class.bytes = class_description.code_super_class;
            type->super_is_code_class = class_description.super_is_code_class;
            type->abstract = class_description.abstract;
            type->transient = class_description.transient;
            type->hide_dropdown = class_description.hide_dropdown;
            type->default_to_instanced = class_description.default_to_instanced;
            type->edit_inline_new = class_description.edit_inline_new;
            type->is_deprecated_class = class_description.deprecated;
            type->config_name.bytes = class_description.config_name;
            type->static_class_global_variable_name.bytes =
                class_description.static_class_global_variable_name;
            type->placeable = class_description.placeable;
            std::int64_t ignored_compose_hash = 0;
            if (class_description.compose_onto_class.find('\0') !=
                    std::string::npos ||
                !compute_processed_code_hash_utf8(
                    class_description.compose_onto_class,
                    ignored_compose_hash)) {
                return fail("class ComposeOnto identity is not serializable");
            }
            type->compose_onto_class_name.bytes =
                class_description.compose_onto_class;
            if (!copy_metadata(
                    class_description.metadata,
                    type->metadata_specifiers,
                    type->metadata_values)) {
                return fail("class metadata is invalid or not serializable");
            }

            for (const preprocessed_property_description& property_description :
                 class_description.properties) {
                precompiled_property* const property =
                    find_property(*type, property_description.property_name);
                if (property == nullptr ||
                    !apply_property(property_description, *property)) {
                    return fail("reflected property descriptor does not map uniquely to output");
                }
            }
            for (const preprocessed_function_description& function_description :
                 class_description.methods) {
                precompiled_function* const function = find_function(
                    type->methods, function_description.script_function_name);
                if (function == nullptr ||
                    !apply_function(function_description, *function)) {
                    return fail("reflected method descriptor does not map uniquely to output");
                }
            }
        }
        if (statics != nullptr) {
            if (staged.statics_class_name.bytes != statics->class_name) {
                return fail("statics-class name does not match its class descriptor");
            }
            for (const preprocessed_function_description& function_description :
                 statics->methods) {
                precompiled_function* const function = find_function(
                    staged.functions, function_description.script_function_name);
                if (function == nullptr ||
                    !apply_function(function_description, *function)) {
                    return fail("reflected global function does not map uniquely to output");
                }
            }
        } else if (!staged.statics_class_name.bytes.empty()) {
            return fail("statics-class name has no matching class descriptor");
        }

        module = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return fail("allocation failed while projecting precompiled metadata");
    } catch (const std::exception& exception) {
        return fail(exception.what());
    } catch (...) {
        return fail("unexpected precompiled metadata projection failure");
    }
}

metadata_projection_result preserve_cached_metadata(
    const precompiled_module& cached,
    precompiled_module& rebuilt) {
    try {
        if (!(cached.module_name == rebuilt.module_name) ||
            cached.classes.size() != rebuilt.classes.size()) {
            return fail("rebuilt cached module does not preserve module/function order");
        }
        precompiled_module staged = rebuilt;
        if (!preserve_function_vector(cached.functions, staged.functions)) {
            return fail("rebuilt cached module does not preserve module/function order");
        }
        staged.code_hash = cached.code_hash;
        staged.imported_modules = cached.imported_modules;
        staged.statics_class_name = cached.statics_class_name;
        staged.declared_events = cached.declared_events;
        staged.declared_delegates = cached.declared_delegates;
        staged.script_relative_filename = cached.script_relative_filename;
        staged.post_init_functions = cached.post_init_functions;
        for (std::size_t class_index = 0U;
             class_index < cached.classes.size(); ++class_index) {
            const precompiled_class& source = cached.classes[class_index];
            precompiled_class& target = staged.classes[class_index];
            if (!(source.class_name == target.class_name) ||
                !(source.name_space == target.name_space) ||
                source.properties.size() != target.properties.size() ||
                source.method_table != target.method_table ||
                !preserve_function_vector(source.methods, target.methods)) {
                return fail("rebuilt cached class does not preserve structural order");
            }
            target.is_in_preprocessor = source.is_in_preprocessor;
            target.super_class = source.super_class;
            target.code_super_class = source.code_super_class;
            target.super_is_code_class = source.super_is_code_class;
            target.abstract = source.abstract;
            target.transient = source.transient;
            target.hide_dropdown = source.hide_dropdown;
            target.default_to_instanced = source.default_to_instanced;
            target.edit_inline_new = source.edit_inline_new;
            target.is_deprecated_class = source.is_deprecated_class;
            target.config_name = source.config_name;
            target.static_class_global_variable_name =
                source.static_class_global_variable_name;
            target.placeable = source.placeable;
            target.metadata_specifiers = source.metadata_specifiers;
            target.metadata_values = source.metadata_values;
            target.compose_onto_class_name = source.compose_onto_class_name;
            for (std::size_t property_index = 0U;
                 property_index < source.properties.size(); ++property_index) {
                const precompiled_property& source_property =
                    source.properties[property_index];
                precompiled_property& target_property =
                    target.properties[property_index];
                if (!(source_property.name == target_property.name)) {
                    return fail("rebuilt cached property order changed");
                }
                target_property.is_unreal_property = source_property.is_unreal_property;
                target_property.metadata_specifiers = source_property.metadata_specifiers;
                target_property.metadata_values = source_property.metadata_values;
                target_property.blueprint_readable = source_property.blueprint_readable;
                target_property.blueprint_writable = source_property.blueprint_writable;
                target_property.edit_const = source_property.edit_const;
                target_property.editable_on_defaults = source_property.editable_on_defaults;
                target_property.editable_on_instance = source_property.editable_on_instance;
                target_property.instanced_reference = source_property.instanced_reference;
                target_property.persistent_instance = source_property.persistent_instance;
                target_property.advanced_display = source_property.advanced_display;
                target_property.transient = source_property.transient;
                target_property.replicated = source_property.replicated;
                target_property.replication_condition = source_property.replication_condition;
                target_property.skip_replication = source_property.skip_replication;
                target_property.skip_serialization = source_property.skip_serialization;
                target_property.save_game = source_property.save_game;
                target_property.rep_notify = source_property.rep_notify;
                target_property.config = source_property.config;
                target_property.interp = source_property.interp;
                target_property.asset_registry_searchable =
                    source_property.asset_registry_searchable;
            }
        }
        rebuilt = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return fail("allocation failed while preserving cached metadata");
    } catch (const std::exception& exception) {
        return fail(exception.what());
    } catch (...) {
        return fail("unexpected cached metadata preservation failure");
    }
}

} // namespace gore::as::standalone::precompiled
