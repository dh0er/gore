#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/module_preprocessor.hpp"

#include "angelscript.h"

#include <cstdint>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

namespace standalone = gore::as::standalone;

namespace {

void message_callback(const asSMessageInfo* message, void*) {
    std::cerr << message->section << ':' << message->row << ':' << message->col
              << ": " << message->message << '\n';
}

int fail(const char* message, asIScriptEngine* engine = nullptr) {
    std::cerr << message << '\n';
    if (engine != nullptr) engine->ShutDownAndRelease();
    return 1;
}

standalone::preprocessor_source source(
    std::string relative_path,
    std::string code) {
    return {
        relative_path,
        "C:/sealed/Script/" + relative_path,
        std::move(code)};
}

bool contains_bytecode(asIScriptModule& module, const char* name) {
    asIScriptFunction* function = module.GetFunctionByName(name);
    if (function == nullptr) return false;
    asUINT length = 0U;
    return function->GetByteCode(&length) != nullptr && length != 0U;
}

} // namespace

int main() {
    standalone::precompiled::cache base_cache;
    standalone::precompiled::precompiled_module base_record;
    base_record.module_name.bytes = "Game.Base";
    standalone::precompiled::precompiled_class base_class;
    base_class.class_name.bytes = "ABase";
    base_class.flags = static_cast<std::int32_t>(asOBJ_REF | asOBJ_NOCOUNT);
    base_class.is_in_preprocessor = true;
    base_class.super_class.bytes = "AActor";
    base_class.code_super_class.bytes = "/Script/Engine.Actor";
    base_record.classes.push_back(std::move(base_class));
    standalone::precompiled::map_string base_key;
    constexpr char base_name[] = "Game.Base";
    base_key.payload.assign(base_name, base_name + sizeof(base_name) - 1U);
    base_cache.modules.emplace_back(std::move(base_key), std::move(base_record));
    std::vector<standalone::preprocessor_base_module> base_modules;
    const auto derived = standalone::derive_preprocessor_base_modules(
        base_cache, base_modules);
    standalone::preprocessor_options base_options;
    base_options.native_super_types = {{
        "AActor",
        "/Script/Engine.Actor",
        128U,
        standalone::native_super_kind::actor,
        false,
    }};
    const auto base_child = standalone::preprocess_lexical_module_graph(
        base_options,
        {source("Game/Child.as", "class AChild : ABase {}\n")},
        base_modules);
    if (!derived.ok || base_modules.size() != 1U ||
        base_modules[0].classes.size() != 1U || !base_child.ok ||
        base_child.modules[0].classes[0].code_super_class !=
            "/Script/Engine.Actor") {
        return fail("decoded cache ancestry did not feed the source frontend");
    }

    standalone::preprocessor_options options;
    options.automatic_imports = false;
    const std::vector<standalone::preprocessor_source> sources = {
        source("Game/Consumer.as", R"AS(import Game.Provider;
int Sum(FPair Value)
{
    return Value.Left + Value.Right;
}
)AS"),
        source("Game/Provider.as", R"AS(USTRUCT()
struct FPair
{
    UPROPERTY()
    int Left;
    int Right;
}
)AS"),
    };
    const auto frontend = standalone::preprocess_lexical_module_graph(options, sources);
    if (!frontend.ok || frontend.modules.size() != 2U ||
        frontend.modules[0].module_name != "Game.Provider" ||
        frontend.modules[1].module_name != "Game.Consumer" ||
        frontend.modules[0].classes.size() != 1U ||
        !frontend.modules[0].classes[0].is_struct ||
        frontend.modules[0].classes[0].properties.size() != 1U) {
        return fail("frontend did not produce the expected module descriptors");
    }

    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return fail("asCreateScriptEngine returned null");
    if (engine->SetMessageCallback(
            asFUNCTION(message_callback), nullptr, asCALL_CDECL) < 0) {
        return fail("SetMessageCallback rejected the diagnostic callback", engine);
    }
    if (engine->RegisterObjectType("UObject", 0, asOBJ_REF | asOBJ_NOCOUNT) < 0) {
        return fail("could not register the synthetic native shadow type", engine);
    }

    standalone::frontend_compile_runtime runtime;
    std::vector<asIScriptModule*> modules;
    const auto compiled = standalone::compile_preprocessed_module_graph(
        *engine, options, frontend, nullptr, runtime, modules);
    if (!compiled.succeeded() || modules.size() != 2U ||
        modules[0]->GetTypeInfoByName("FPair") == nullptr ||
        !contains_bytecode(*modules[1], "Sum")) {
        return fail("frontend descriptors did not compile through the graph bridge", engine);
    }

    standalone::preprocessor_options native_options;
    native_options.native_super_types = {{
        "UObject",
        "/Script/CoreUObject.Object",
        24U,
        standalone::native_super_kind::other_uobject,
        false,
    }};
    standalone::lexical_preprocess_result native_frontend;
    native_frontend.ok = true;
    standalone::lexical_module_description native_module;
    native_module.module_name = "Game.NativeChild";
    native_module.code.push_back({
        "Game/NativeChild.as",
        "C:/sealed/Script/Game/NativeChild.as",
        "class AChild { int Value; }\n",
    });
    standalone::preprocessed_class_description native_class;
    native_class.class_name = "AChild";
    native_class.super_class = "UObject";
    native_class.code_super_class = "/Script/CoreUObject.Object";
    native_class.super_is_code_class = true;
    native_module.classes.push_back(std::move(native_class));
    native_frontend.modules.push_back(std::move(native_module));
    std::vector<asIScriptModule*> native_modules;
    const auto native_compiled = standalone::compile_preprocessed_module_graph(
        *engine,
        native_options,
        native_frontend,
        nullptr,
        runtime,
        native_modules);
    if (!native_compiled.succeeded() || native_modules.size() != 1U ||
        native_modules[0]->GetTypeInfoByName("AChild") == nullptr) {
        return fail("native shadow pre-class data did not reach the graph builder", engine);
    }

    const auto broken_frontend = standalone::preprocess_lexical_module_graph(
        options,
        {source("Bad/Broken.as", "int Broken( {\n")});
    std::vector<asIScriptModule*> preserved = modules;
    const auto broken = standalone::compile_preprocessed_module_graph(
        *engine, options, broken_frontend, nullptr, runtime, preserved);
    if (broken.succeeded() ||
        broken.phase != standalone::frontend_compile_phase::build_graph ||
        engine->GetModule("Bad.Broken", asGM_ONLY_IF_EXISTS) != nullptr ||
        preserved != modules) {
        return fail("failed frontend graph was not discarded atomically", engine);
    }

    std::cout << "G1R frontend bridge compiled preprocessed descriptors and cleaned up "
                 "a rejected graph\n";
    engine->ShutDownAndRelease();
    return 0;
}
