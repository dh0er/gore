#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/module_preprocessor.hpp"

#include "AngelscriptManager.h"
#include "angelscript.h"
#include "as_scriptfunction.h"

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

    asIScriptEngine* automatic_engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (automatic_engine == nullptr) {
        return fail("automatic-import engine creation returned null", engine);
    }
    if (automatic_engine->SetMessageCallback(
            asFUNCTION(message_callback), nullptr, asCALL_CDECL) < 0 ||
        automatic_engine->SetEngineProperty(asEP_AUTOMATIC_IMPORTS, 1) < 0) {
        automatic_engine->ShutDownAndRelease();
        return fail("automatic-import engine configuration failed", engine);
    }
    standalone::preprocessor_options automatic_options;
    automatic_options.automatic_imports = true;
    automatic_options.flags = {{"RELEASE", true}};
    const auto automatic_frontend = standalone::preprocess_lexical_module_graph(
        automatic_options,
        {source("Graph/ClosureProvider.as", "int ClosureValue() { return 42; }\n"),
         source("Graph/ClosureMiddle.as", "int MiddleValue() { return ClosureValue(); }\n"),
         source("Graph/ClosureConsumer.as", R"AS(#if RELEASE
int ConsumerValue() { return MiddleValue(); }
#endif
)AS")});
    std::vector<asIScriptModule*> automatic_modules;
    const auto automatic_compiled = standalone::compile_preprocessed_module_graph(
        *automatic_engine,
        automatic_options,
        automatic_frontend,
        nullptr,
        runtime,
        automatic_modules);
    if (!automatic_frontend.ok || !automatic_compiled.succeeded() ||
        automatic_modules.size() != 3U ||
        !contains_bytecode(*automatic_modules[2], "ConsumerValue")) {
        automatic_engine->ShutDownAndRelease();
        return fail("automatic-import transitive graph closure did not compile", engine);
    }
    automatic_engine->ShutDownAndRelease();

    FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode = true;
    standalone::preprocessor_options editor_options;
    editor_options.automatic_imports = false;
    editor_options.flags = {{"EDITOR", true}};
    const auto invalid_editor_use = standalone::preprocess_lexical_module_graph(
        editor_options,
        {source("Game/EditorConsumer.as", R"AS(import Editor.Tools;
int UseEditorTool() { return EditorValue(); }
)AS"),
         source("Editor/Tools.as", "int EditorValue() { return 9; }\n")});
    std::vector<asIScriptModule*> invalid_editor_modules;
    const auto invalid_editor_compile = standalone::compile_preprocessed_module_graph(
        *engine,
        editor_options,
        invalid_editor_use,
        nullptr,
        runtime,
        invalid_editor_modules);
    if (invalid_editor_compile.succeeded() ||
        invalid_editor_compile.phase !=
            standalone::frontend_compile_phase::build_graph ||
        engine->GetModule("Editor.Tools", asGM_ONLY_IF_EXISTS) != nullptr ||
        engine->GetModule("Game.EditorConsumer", asGM_ONLY_IF_EXISTS) != nullptr) {
        return fail("editor-only module usage was not rejected and cleaned up", engine);
    }

    const auto valid_editor_use = standalone::preprocess_lexical_module_graph(
        editor_options,
        {source("Game/EditorSafe.as", R"AS(import Game.EditorProvider;
#if EDITOR
int UseEditorValue() { return EditorValue(); }
#endif
)AS"),
         source("Game/EditorProvider.as", R"AS(#if EDITOR
int EditorValue() { return 11; }
#endif
int AfterEditorValue() { return 12; }
)AS")});
    std::vector<asIScriptModule*> editor_modules;
    const auto editor_compiled = standalone::compile_preprocessed_module_graph(
        *engine,
        editor_options,
        valid_editor_use,
        nullptr,
        runtime,
        editor_modules);
    if (!editor_compiled.succeeded() || editor_modules.size() != 2U ||
        valid_editor_use.modules[0].editor_only_blocks.size() != 1U ||
        valid_editor_use.modules[1].editor_only_blocks.size() != 1U ||
        !contains_bytecode(*editor_modules[0], "EditorValue") ||
        !contains_bytecode(*editor_modules[0], "AfterEditorValue") ||
        !contains_bytecode(*editor_modules[1], "UseEditorValue")) {
        return fail("editor-only line barriers did not reach the pinned builder", engine);
    }
    auto* const editor_function = static_cast<asCScriptFunction*>(
        editor_modules[0]->GetFunctionByName("EditorValue"));
    auto* const after_editor_function = static_cast<asCScriptFunction*>(
        editor_modules[0]->GetFunctionByName("AfterEditorValue"));
    if (editor_function == nullptr || after_editor_function == nullptr ||
        !editor_function->traits.GetTrait(asTRAIT_EDITOR_ONLY) ||
        after_editor_function->traits.GetTrait(asTRAIT_EDITOR_ONLY)) {
        return fail("editor-only block end included the first following function", engine);
    }

    standalone::preprocessor_options native_options;
    native_options.native_super_types = {{
        "UObject",
        "/Script/CoreUObject.Object",
        24U,
        standalone::native_super_kind::other_uobject,
        false,
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
