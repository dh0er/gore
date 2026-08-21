#include "gore_as_standalone/core.hpp"

#include "as_builder.h"
#include "as_module.h"
#include "as_scriptengine.h"

namespace gore::as::standalone {

int build_module(asIScriptModule& module_interface) {
    auto& module = static_cast<asCModule&>(module_interface);
    asCScriptEngine* engine = module.engine;

    if (module.HasExternalReferences(false)) {
        return asMODULE_IS_IN_USE;
    }

    int result = engine->RequestBuild();
    if (result < 0) {
        return result;
    }

    engine->PrepareEngine();
    if (engine->configFailed) {
        engine->BuildCompleted();
        return asINVALID_CONFIGURATION;
    }

    module.InternalReset();
    if (module.builder == nullptr) {
        engine->BuildCompleted();
        return asSUCCESS;
    }

    asCBuilder* builder = module.builder;
    result = builder->BuildParallelParseScripts();
    if (result >= 0) {
        result = builder->BuildGenerateTypes();
    }
    if (result >= 0) {
        result = builder->BuildGenerateFunctions();
    }
    if (result >= 0) {
        result = builder->BuildLayoutClasses();
    }
    if (result >= 0) {
        builder->BuildAllocateGlobalVariables();
        result = builder->BuildLayoutFunctions();
    }
    if (result >= 0) {
        result = builder->BuildCompileCode();
    }

    asDELETE(module.builder, asCBuilder);
    module.builder = nullptr;

    if (result < 0) {
        module.InternalReset();
    } else {
        module.JITCompile();
        engine->PrepareEngine();
    }
    engine->BuildCompleted();
    return result;
}

} // namespace gore::as::standalone
