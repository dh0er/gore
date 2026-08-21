#include "gore_as_standalone/frontend_compile.hpp"

#include "as_module.h"
#include "as_scriptengine.h"

#include <algorithm>
#include <cstdint>
#include <exception>
#include <new>
#include <unordered_map>
#include <unordered_set>
#include <utility>

namespace gore::as::standalone {

struct frontend_compile_runtime::impl {
    std::uint8_t delegate_tag = 0U;
    std::uint8_t multicast_delegate_tag = 0U;
};

namespace {

frontend_compile_result failure(
    const frontend_compile_phase phase,
    const std::size_t module_index,
    std::string detail,
    const int code = asERROR) {
    frontend_compile_result result;
    result.code = code < 0 ? code : asERROR;
    result.phase = phase;
    result.module_index = module_index;
    result.detail = std::move(detail);
    return result;
}

class module_cleanup final {
public:
    explicit module_cleanup(asIScriptEngine& engine) noexcept : engine_(engine) {}
    ~module_cleanup() {
        if (keep_) return;
        for (auto name = names_.rbegin(); name != names_.rend(); ++name) {
            engine_.DiscardModule(name->c_str());
        }
    }

    void add(std::string name) { names_.push_back(std::move(name)); }
    void keep() noexcept { keep_ = true; }

private:
    asIScriptEngine& engine_;
    std::vector<std::string> names_;
    bool keep_ = false;
};

struct classify_context {
    registry_runtime* registry = nullptr;
    const lexical_preprocess_result* input = nullptr;
};

asITypeInfo* find_shadow_type(
    asIScriptEngine& engine_interface,
    const std::string& angelscript_name) noexcept {
    auto& engine = static_cast<asCScriptEngine&>(engine_interface);
    return engine.allRegisteredTypesByName.FindFirst_CaseInsensitive(
        angelscript_name.c_str());
}

int classify_delegates(
    void* const raw_context,
    asIScriptModule* const* const modules,
    const std::size_t module_count) noexcept {
    if (raw_context == nullptr || modules == nullptr) return asINVALID_ARG;
    auto& context = *static_cast<classify_context*>(raw_context);
    if (context.input == nullptr || context.input->modules.size() != module_count) {
        return asINVALID_ARG;
    }
    try {
        for (std::size_t index = 0U; index < module_count; ++index) {
            const lexical_module_description& description = context.input->modules[index];
            for (const preprocessed_delegate_description& delegate : description.delegates) {
                if (context.registry == nullptr) return asINVALID_CONFIGURATION;
                const std::string declaration = delegate.name_space.empty()
                    ? delegate.delegate_name
                    : delegate.name_space + "::" + delegate.delegate_name;
                asITypeInfo* const type = modules[index]->GetTypeInfoByDecl(declaration.c_str());
                if (type == nullptr ||
                    !classify_dynamic_script_type(
                        *context.registry,
                        *type,
                        delegate.multicast
                            ? dynamic_script_type_category::multicast_delegate
                            : dynamic_script_type_category::delegate)) {
                    return asERROR;
                }
            }
        }
        return asSUCCESS;
    } catch (...) {
        return asERROR;
    }
}

} // namespace

base_descriptor_result derive_preprocessor_base_modules(
    const precompiled::cache& input,
    std::vector<preprocessor_base_module>& modules) {
    try {
        if (input.modules.size() > max_preprocessor_base_modules) {
            return {false, no_failed_module, "cache has too many base modules"};
        }
        std::vector<preprocessor_base_module> staged;
        staged.reserve(input.modules.size());
        std::unordered_set<std::string> module_names;
        std::unordered_set<std::string> class_names;
        std::size_t class_count = 0U;
        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            const precompiled::precompiled_module& encoded = input.modules[index].second;
            if (encoded.module_name.bytes.empty() ||
                !module_names.insert(encoded.module_name.bytes).second) {
                return {false, index, "cache has an empty or duplicate base module name"};
            }
            preprocessor_base_module module;
            module.module_name = encoded.module_name.bytes;
            for (const precompiled::precompiled_class& type : encoded.classes) {
                if (!type.is_in_preprocessor) continue;
                if (class_count == max_preprocessor_base_classes) {
                    return {false, index, "cache has too many preprocessor base classes"};
                }
                ++class_count;
                if (type.class_name.bytes.empty() ||
                    !class_names.insert(type.class_name.bytes).second) {
                    return {false, index, "cache has an empty or duplicate base class name"};
                }
                preprocessor_base_class description;
                description.class_name = type.class_name.bytes;
                description.name_space = type.name_space.bytes;
                description.super_class = type.super_class.bytes;
                description.code_super_class = type.code_super_class.bytes;
                description.super_is_code_class = type.super_is_code_class;
                description.is_struct =
                    (static_cast<asDWORD>(type.flags) & asOBJ_VALUE) != 0U;
                if (!description.is_struct && description.code_super_class.empty()) {
                    return {
                        false,
                        index,
                        "preprocessor base class has no serialized native code superclass"};
                }
                module.classes.push_back(std::move(description));
            }
            staged.push_back(std::move(module));
        }
        modules = std::move(staged);
        return {};
    } catch (const std::bad_alloc&) {
        return {false, no_failed_module, "allocation failed while deriving base descriptors"};
    } catch (const std::exception& exception) {
        return {false, no_failed_module, exception.what()};
    } catch (...) {
        return {false, no_failed_module, "unexpected base descriptor derivation failure"};
    }
}

frontend_compile_runtime::frontend_compile_runtime() : impl_(std::make_unique<impl>()) {}
frontend_compile_runtime::~frontend_compile_runtime() = default;
frontend_compile_runtime::frontend_compile_runtime(frontend_compile_runtime&&) noexcept = default;
frontend_compile_runtime& frontend_compile_runtime::operator=(
    frontend_compile_runtime&&) noexcept = default;

frontend_compile_result compile_preprocessed_module_graph(
    asIScriptEngine& engine,
    const preprocessor_options& options,
    const lexical_preprocess_result& input,
    registry_runtime* const registry,
    frontend_compile_runtime& runtime,
    std::vector<asIScriptModule*>& modules) {
    try {
        if (runtime.impl_ == nullptr) {
            return failure(
                frontend_compile_phase::preflight,
                no_failed_module,
                "frontend runtime was moved from",
                asINVALID_ARG);
        }
        if (!input.ok || std::any_of(
                input.diagnostics.begin(), input.diagnostics.end(),
                [](const preprocessor_diagnostic& diagnostic) {
                    return diagnostic.severity == preprocessor_diagnostic_severity::error;
                })) {
            return failure(
                frontend_compile_phase::preflight,
                no_failed_module,
                "frontend input contains preprocessing errors",
                asINVALID_ARG);
        }
        if (input.modules.size() > max_preprocessor_sources) {
            return failure(
                frontend_compile_phase::preflight,
                no_failed_module,
                "frontend module count exceeds the bounded maximum",
                asINVALID_ARG);
        }

        std::unordered_map<std::string, const native_super_type*> native_by_path;
        native_by_path.reserve(options.native_super_types.size());
        for (const native_super_type& native : options.native_super_types) {
            if (native.unreal_class_path.empty() ||
                !native_by_path.emplace(native.unreal_class_path, &native).second) {
                return failure(
                    frontend_compile_phase::preflight,
                    no_failed_module,
                    "native-super profile has an empty or duplicate Unreal class path",
                    asINVALID_ARG);
            }
        }

        std::unordered_set<std::string> overlay_names;
        overlay_names.reserve(input.modules.size());
        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            const lexical_module_description& description = input.modules[index];
            if (description.module_name.empty() ||
                !overlay_names.insert(description.module_name).second) {
                return failure(
                    frontend_compile_phase::preflight,
                    index,
                    "frontend contains an empty or duplicate module name",
                    asINVALID_ARG);
            }
            if (engine.GetModule(description.module_name.c_str(), asGM_ONLY_IF_EXISTS) != nullptr) {
                return failure(
                    frontend_compile_phase::preflight,
                    index,
                    "overlay module already exists in the target engine",
                    asNAME_TAKEN);
            }
            if (!description.delegates.empty() && registry == nullptr) {
                return failure(
                    frontend_compile_phase::preflight,
                    index,
                    "delegate compilation requires the live registry runtime",
                    asINVALID_CONFIGURATION);
            }
            for (const preprocessed_class_description& type : description.classes) {
                if (type.is_struct || type.code_super_class.empty()) continue;
                const auto native = native_by_path.find(type.code_super_class);
                if (native == native_by_path.end()) {
                    return failure(
                        frontend_compile_phase::preflight,
                        index,
                        "class " + type.class_name +
                            " has an unprofiled native code superclass",
                        asINVALID_CONFIGURATION);
                }
                if (find_shadow_type(
                        engine, native->second->angelscript_type_name) == nullptr) {
                    return failure(
                        frontend_compile_phase::preflight,
                        index,
                        "registered shadow type is absent from the target engine",
                        asINVALID_CONFIGURATION);
                }
            }
        }
        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            for (const std::string& imported : input.modules[index].imported_modules) {
                if (overlay_names.find(imported) == overlay_names.end() &&
                    engine.GetModule(imported.c_str(), asGM_ONLY_IF_EXISTS) == nullptr) {
                    return failure(
                        frontend_compile_phase::preflight,
                        index,
                        "explicitly imported module is absent from the target engine",
                        asNO_MODULE);
                }
            }
        }

        module_cleanup cleanup(engine);
        std::vector<asIScriptModule*> created;
        created.reserve(input.modules.size());
        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            const lexical_module_description& description = input.modules[index];
            asIScriptModule* const module =
                engine.GetModule(description.module_name.c_str(), asGM_ALWAYS_CREATE);
            if (module == nullptr) {
                return failure(
                    frontend_compile_phase::create_modules,
                    index,
                    "engine refused to create an overlay module");
            }
            created.push_back(module);
            static_cast<asCModule*>(module)->baseModuleName =
                description.module_name.c_str();
            cleanup.add(description.module_name);
        }

        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            asIScriptModule& module = *created[index];
            const lexical_module_description& description = input.modules[index];
            for (const preprocessed_class_description& type : description.classes) {
                if (type.is_struct || type.code_super_class.empty()) continue;
                const native_super_type& native = *native_by_path.at(type.code_super_class);
                asPreClassData data;
                data.PropertyOffset = static_cast<std::size_t>(native.property_offset);
                data.ShadowType = find_shadow_type(engine, native.angelscript_type_name);
                module.AddPreClassData(type.class_name.c_str(), data);
            }
            for (const preprocessed_delegate_description& delegate : description.delegates) {
                asPreClassData data;
                data.InitialUserData = delegate.multicast
                    ? static_cast<void*>(&runtime.impl_->multicast_delegate_tag)
                    : static_cast<void*>(&runtime.impl_->delegate_tag);
                module.AddPreClassData(delegate.delegate_name.c_str(), data);
            }
        }

        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            for (const std::string& imported : input.modules[index].imported_modules) {
                asIScriptModule* dependency = nullptr;
                const auto overlay = std::find_if(
                    input.modules.begin(), input.modules.end(),
                    [&imported](const lexical_module_description& candidate) {
                        return candidate.module_name == imported;
                    });
                if (overlay != input.modules.end()) {
                    dependency = created[static_cast<std::size_t>(
                        std::distance(input.modules.begin(), overlay))];
                } else {
                    dependency = engine.GetModule(imported.c_str(), asGM_ONLY_IF_EXISTS);
                }
                if (dependency == nullptr) {
                    return failure(
                        frontend_compile_phase::import_modules,
                        index,
                        "explicit import disappeared after preflight",
                        asNO_MODULE);
                }
                created[index]->ImportModule(dependency);
            }
        }

        for (std::size_t index = 0U; index < input.modules.size(); ++index) {
            for (const preprocessed_code_section& section : input.modules[index].code) {
                const int added = created[index]->AddScriptSection(
                    section.absolute_path.c_str(),
                    section.conditioned_code.data(),
                    section.conditioned_code.size());
                if (added < 0) {
                    return failure(
                        frontend_compile_phase::add_sections,
                        index,
                        "engine rejected a preprocessed source section",
                        added);
                }
            }
        }

        classify_context classify{registry, &input};
        const graph_build_hooks hooks{&classify, &classify_delegates};
        const graph_build_result graph =
            build_module_graph(created.data(), created.size(), &hooks);
        if (!graph.succeeded()) {
            frontend_compile_result result = failure(
                frontend_compile_phase::build_graph,
                graph.failed_module,
                "graph-wide compilation rejected the preprocessed modules",
                graph.code);
            result.graph = graph;
            return result;
        }

        modules = created;
        cleanup.keep();
        return {};
    } catch (const std::bad_alloc&) {
        return failure(
            frontend_compile_phase::cleanup,
            no_failed_module,
            "allocation failed in frontend compiler bridge",
            asOUT_OF_MEMORY);
    } catch (const std::exception& exception) {
        return failure(
            frontend_compile_phase::cleanup,
            no_failed_module,
            exception.what());
    } catch (...) {
        return failure(
            frontend_compile_phase::cleanup,
            no_failed_module,
            "unexpected frontend compiler bridge failure");
    }
}

} // namespace gore::as::standalone
