#include "gore_as_standalone/core.hpp"

#include "as_builder.h"
#include "as_module.h"
#include "as_objecttype.h"
#include "as_scriptengine.h"

#include <vector>

namespace gore::as::standalone {
namespace {

struct module_state {
    asCModule* module = nullptr;
    bool failed = false;
};

class build_session final {
public:
    explicit build_session(asCScriptEngine& engine) noexcept
        : engine_(engine),
          previous_defer_validation_(engine.deferValidationOfTemplateTypes),
          previous_defer_size_(engine.deferCalculatingTemplateSize) {
        engine_.deferValidationOfTemplateTypes = true;
        engine_.deferCalculatingTemplateSize = true;
    }

    ~build_session() {
        engine_.deferValidationOfTemplateTypes = previous_defer_validation_;
        engine_.deferCalculatingTemplateSize = previous_defer_size_;
        if (build_requested_) {
            engine_.BuildCompleted();
        }
    }

    build_session(const build_session&) = delete;
    build_session& operator=(const build_session&) = delete;

    int request() noexcept {
        const int result = engine_.RequestBuild();
        build_requested_ = result >= 0;
        return result;
    }

private:
    asCScriptEngine& engine_;
    bool previous_defer_validation_;
    bool previous_defer_size_;
    bool build_requested_ = false;
};

class graph_cleanup final {
public:
    explicit graph_cleanup(std::vector<module_state>& modules) noexcept : modules_(modules) {}

    ~graph_cleanup() {
        for (auto state = modules_.rbegin(); state != modules_.rend(); ++state) {
            if (state->module->builder != nullptr) {
                asDELETE(state->module->builder, asCBuilder);
                state->module->builder = nullptr;
            }
        }
        if (!keep_built_modules_) {
            for (auto state = modules_.rbegin(); state != modules_.rend(); ++state) {
                state->module->InternalReset();
            }
        }
    }

    graph_cleanup(const graph_cleanup&) = delete;
    graph_cleanup& operator=(const graph_cleanup&) = delete;

    void keep_built_modules() noexcept { keep_built_modules_ = true; }

private:
    std::vector<module_state>& modules_;
    bool keep_built_modules_ = false;
};

void record_failure(
    graph_build_result& result,
    module_state& state,
    const std::size_t module_index,
    const graph_build_phase phase,
    const int code) noexcept {
    state.failed = true;
    if (result.succeeded()) {
        result.code = code < 0 ? code : asERROR;
        result.phase = phase;
        result.failed_module = module_index;
    }
}

void record_graph_failure(
    graph_build_result& result,
    const graph_build_phase phase,
    const int code) noexcept {
    if (result.succeeded()) {
        result.code = code < 0 ? code : asERROR;
        result.phase = phase;
        result.failed_module = no_failed_module;
    }
}

} // namespace

graph_build_result build_module_graph(
    asIScriptModule* const* module_interfaces,
    const std::size_t module_count,
    const graph_build_hooks* const hooks) {
    graph_build_result result{};
    if (module_count == 0U) {
        return result;
    }
    if (module_interfaces == nullptr) {
        result.code = asINVALID_ARG;
        result.phase = graph_build_phase::input_validation;
        return result;
    }

    std::vector<module_state> modules;
    modules.reserve(module_count);
    asCScriptEngine* engine = nullptr;
    for (std::size_t index = 0U; index < module_count; ++index) {
        if (module_interfaces[index] == nullptr) {
            result.code = asINVALID_ARG;
            result.phase = graph_build_phase::input_validation;
            result.failed_module = index;
            return result;
        }

        auto* module = static_cast<asCModule*>(module_interfaces[index]);
        if (engine == nullptr) {
            engine = module->engine;
        } else if (module->engine != engine) {
            result.code = asINVALID_ARG;
            result.phase = graph_build_phase::input_validation;
            result.failed_module = index;
            return result;
        }
        for (const module_state& existing : modules) {
            if (existing.module == module) {
                result.code = asINVALID_ARG;
                result.phase = graph_build_phase::input_validation;
                result.failed_module = index;
                return result;
            }
        }
        if (module->HasExternalReferences(false)) {
            result.code = asMODULE_IS_IN_USE;
            result.phase = graph_build_phase::input_validation;
            result.failed_module = index;
            return result;
        }
        modules.push_back({module, false});
    }

    build_session session(*engine);
    result.code = session.request();
    if (result.code < 0) {
        result.phase = graph_build_phase::request_build;
        return result;
    }

    engine->PrepareEngine();
    if (engine->configFailed) {
        result.code = asINVALID_CONFIGURATION;
        result.phase = graph_build_phase::prepare_engine;
        return result;
    }

    for (module_state& state : modules) {
        state.module->InternalReset();
    }
    graph_cleanup cleanup(modules);

    // Pinned FAngelscriptManager barrier 1: parse every module before any
    // module publishes its type declarations.
    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildParallelParseScripts();
        if (phase_result != asSUCCESS) {
            record_failure(result, state, index, graph_build_phase::parse_scripts, phase_result);
        }
    }

    // Barrier 2: register all module-local type names. A consumer's function
    // declarations may then reference a type from a module later in the list.
    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.failed || state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildGenerateTypes();
        if (phase_result != asSUCCESS) {
            record_failure(result, state, index, graph_build_phase::generate_types, phase_result);
        }
    }

    if (result.succeeded() && hooks != nullptr &&
        hooks->after_generate_types != nullptr) {
        const int hook_result = hooks->after_generate_types(
            hooks->context, module_interfaces, module_count);
        if (hook_result != asSUCCESS) {
            record_graph_failure(
                result, graph_build_phase::post_generate_types, hook_result);
        }
    }

    // Barrier 3: generate declarations and class bodies only after every
    // successful module has exposed its type names.
    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.failed || state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildGenerateFunctions();
        if (phase_result != asSUCCESS) {
            record_failure(
                result, state, index, graph_build_phase::generate_functions, phase_result);
        }
    }

    // Manager parity: class layout is attempted for every source builder,
    // including a builder that already holds diagnostics from an earlier phase.
    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildLayoutClasses();
        if (phase_result != asSUCCESS) {
            record_failure(result, state, index, graph_build_phase::layout_classes, phase_result);
        }
    }

    engine->deferCalculatingTemplateSize = false;
    for (asCObjectType* instance : engine->unvalidatedTemplateInstances) {
        instance->CalculateTemplateSize();
    }

    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.failed || state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildLayoutFunctions();
        if (phase_result != asSUCCESS) {
            record_failure(result, state, index, graph_build_phase::layout_functions, phase_result);
        }
    }

    // Stage 3 owns builder destruction in FAngelscriptManager. Run the code
    // pass for every source builder so its accumulated diagnostics are closed
    // out, then release it immediately and perform the optional JIT hook.
    for (std::size_t index = 0U; index < modules.size(); ++index) {
        module_state& state = modules[index];
        if (state.module->builder == nullptr) {
            continue;
        }
        const int phase_result = state.module->builder->BuildCompileCode();
        if (phase_result != asSUCCESS) {
            record_failure(result, state, index, graph_build_phase::compile_code, phase_result);
        }
        asDELETE(state.module->builder, asCBuilder);
        state.module->builder = nullptr;
        state.module->JITCompile();
    }

    // Manager stage 4 validates templates once for the whole graph.
    asCBuilder template_validator(engine, nullptr);
    template_validator.Reset();
    template_validator.EvaluateTemplateInstances(false);
    engine->deferValidationOfTemplateTypes = false;
    if (template_validator.numErrors > 0) {
        record_graph_failure(result, graph_build_phase::validate_template_instances, asERROR);
    }

    if (result.succeeded()) {
        for (std::size_t index = 0U; index < modules.size(); ++index) {
            const int phase_result = modules[index].module->ResetGlobalVars(nullptr);
            if (phase_result != asSUCCESS) {
                record_failure(
                    result,
                    modules[index],
                    index,
                    graph_build_phase::initialize_globals,
                    phase_result);
            }
        }
    }

    if (result.succeeded()) {
        cleanup.keep_built_modules();
    }
    return result;
}

int build_module(asIScriptModule& module) {
    asIScriptModule* modules[] = {&module};
    return build_module_graph(modules, 1U).code;
}

} // namespace gore::as::standalone
