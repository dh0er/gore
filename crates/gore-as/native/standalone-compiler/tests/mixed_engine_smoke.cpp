#include "gore_as_standalone/core.hpp"
#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/precompiled_engine.hpp"

#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

namespace standalone = gore::as::standalone;
namespace precompiled = gore::as::standalone::precompiled;

namespace {

void message_callback(const asSMessageInfo* message, void*) {
    std::cerr << message->section << ':' << message->row << ':' << message->col
              << ": " << message->message << '\n';
}

std::int32_t skew_function() { return 0; }

struct dummy_value {
    std::int32_t value = 0;
};

precompiled::map_string module_key(const char* const name) {
    const std::string text(name);
    return {false, std::vector<std::uint8_t>(text.begin(), text.end())};
}

bool execute_no_args(
    asIScriptEngine& engine,
    asIScriptModule& module,
    const char* const name,
    const asDWORD expected) {
    asIScriptFunction* const function = module.GetFunctionByName(name);
    asIScriptContext* const context = engine.CreateContext();
    const bool prepared = function != nullptr && context != nullptr &&
                          context->Prepare(function) >= 0;
    const int execution = prepared ? context->Execute() : asERROR;
    const asDWORD actual = context == nullptr ? 0U : context->GetReturnDWord();
    if (context != nullptr) context->Release();
    if (execution != asEXECUTION_FINISHED || actual != expected) {
        std::cerr << name << " returned " << actual
                  << " with execution state " << execution << '\n';
        return false;
    }
    return true;
}

bool execute_add(
    asIScriptEngine& engine,
    asIScriptModule& module,
    const asDWORD expected) {
    asIScriptFunction* const function = module.GetFunctionByName("Add");
    asIScriptContext* const context = engine.CreateContext();
    const bool prepared = function != nullptr && context != nullptr &&
                          context->Prepare(function) >= 0 &&
                          context->SetArgDWord(0U, 20U) >= 0 &&
                          context->SetArgDWord(1U, 22U) >= 0;
    const int execution = prepared ? context->Execute() : asERROR;
    const asDWORD actual = context == nullptr ? 0U : context->GetReturnDWord();
    if (context != nullptr) context->Release();
    return execution == asEXECUTION_FINISHED && actual == expected;
}

standalone::lexical_module_description source_module(
    std::string name,
    std::string path,
    std::string code,
    std::vector<std::string> imports = {}) {
    standalone::lexical_module_description module;
    module.module_name = std::move(name);
    module.imported_modules = std::move(imports);
    standalone::preprocessed_code_section section;
    section.relative_path = path;
    section.absolute_path = std::move(path);
    section.conditioned_code = std::move(code);
    module.code.push_back(std::move(section));
    return module;
}

} // namespace

int main() {
    asIScriptEngine* const source_engine = asCreateScriptEngine();
    if (source_engine == nullptr) return 1;
    source_engine->SetMessageCallback(
        asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    asIScriptModule* const provider =
        source_engine->GetModule("Provider", asGM_ALWAYS_CREATE);
    asIScriptModule* const consumer =
        source_engine->GetModule("Consumer", asGM_ALWAYS_CREATE);
    constexpr char provider_code[] =
        "struct SharedValue { int Number; }\n"
        "int Add(int Left, int Right) { return Left + Right; }";
    constexpr char consumer_code[] =
        "int CallProvider() { return Add(20, 22); }\n"
        "int ReadProviderValue() { SharedValue Value; Value.Number = 42; return Value.Number; }";
    consumer->ImportModule(provider);
    if (provider->AddScriptSection(
            "Provider.as", provider_code, sizeof(provider_code) - 1U) < 0 ||
        consumer->AddScriptSection(
            "Consumer.as", consumer_code, sizeof(consumer_code) - 1U) < 0) {
        source_engine->ShutDownAndRelease();
        return 2;
    }
    asIScriptModule* source_graph[] = {provider, consumer};
    const auto source_build = standalone::build_module_graph(source_graph, 2U);
    asIScriptFunction* const original_add = provider->GetFunctionByName("Add");
    asITypeInfo* const original_value = provider->GetTypeInfoByDecl("SharedValue");
    if (!source_build.succeeded() || original_add == nullptr ||
        original_value == nullptr ||
        !execute_no_args(*source_engine, *consumer, "CallProvider", 42U) ||
        !execute_no_args(*source_engine, *consumer, "ReadProviderValue", 42U)) {
        std::cerr << "source fixture graph did not compile\n";
        source_engine->ShutDownAndRelease();
        return 3;
    }
    const int original_add_id = original_add->GetId();
    const int original_value_id = original_value->GetTypeId();

    precompiled::cache cache;
    precompiled::precompiled_module encoded_provider;
    precompiled::precompiled_module encoded_consumer;
    auto exported = precompiled::export_module_checkpoint(
        *provider, module_key("Provider"), {"Provider.as"},
        1, encoded_provider, &cache);
    if (exported.succeeded()) {
        exported = precompiled::export_module_checkpoint(
            *consumer, module_key("Consumer"), {"Consumer.as"},
            2, encoded_consumer, &cache);
    }
    if (!exported.succeeded()) {
        std::cerr << "mixed fixture export failed: " << exported.detail << '\n';
        source_engine->ShutDownAndRelease();
        return 4;
    }
    cache.modules.emplace_back(module_key("Provider"), std::move(encoded_provider));
    cache.modules.emplace_back(module_key("Consumer"), std::move(encoded_consumer));

    standalone::lexical_preprocess_result overlays;
    overlays.ok = true;
    overlays.modules.push_back(source_module(
        "Provider", "Provider.as",
        "struct SharedValue { int Padding; int Number; }\n"
        "int Add(int Left, int Right) { return Left + Right + 1; }"));
    overlays.modules.push_back(source_module(
        "Addon", "Addon.as",
        "int Added() { return CallProvider(); }", {"Consumer"}));

    asIScriptEngine* const target_engine = asCreateScriptEngine();
    target_engine->SetMessageCallback(
        asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    if (target_engine->RegisterGlobalFunction(
            "int Skew()", asFUNCTION(skew_function), asCALL_CDECL) < 0 ||
        target_engine->RegisterObjectType(
            "Dummy", sizeof(dummy_value),
            asOBJ_VALUE | asOBJ_POD | asGetTypeTraits<dummy_value>()) < 0) {
        source_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        return 5;
    }
    standalone::frontend_compile_runtime runtime;
    standalone::preprocessor_options options;
    std::vector<asIScriptModule*> built;
    const auto mixed = precompiled::compile_mixed_cache_checkpoint(
        *target_engine, cache, options, overlays, nullptr, runtime, built);
    asIScriptModule* const built_provider =
        target_engine->GetModule("Provider", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_consumer =
        target_engine->GetModule("Consumer", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_addon =
        target_engine->GetModule("Addon", asGM_ONLY_IF_EXISTS);
    asIScriptFunction* const replacement_add = built_provider == nullptr
        ? nullptr
        : built_provider->GetFunctionByName("Add");
    asITypeInfo* const replacement_value = built_provider == nullptr
        ? nullptr
        : built_provider->GetTypeInfoByDecl("SharedValue");
    if (!mixed.succeeded() || built.size() != 3U ||
        built[0] != built_provider || built[1] != built_consumer ||
        built[2] != built_addon || replacement_add == nullptr ||
        replacement_value == nullptr ||
        replacement_add->GetId() == original_add_id ||
        replacement_value->GetTypeId() == original_value_id ||
        !execute_add(*target_engine, *built_provider, 43U) ||
        !execute_no_args(*target_engine, *built_consumer, "CallProvider", 43U) ||
        !execute_no_args(
            *target_engine, *built_consumer, "ReadProviderValue", 42U) ||
        !execute_no_args(*target_engine, *built_addon, "Added", 43U)) {
        std::cerr << "mixed edit/add graph failed: phase="
                  << static_cast<int>(mixed.phase) << "; module="
                  << mixed.module_index << "; detail=" << mixed.detail << '\n';
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 6;
    }

    asIScriptEngine* const rejected_engine = asCreateScriptEngine();
    standalone::lexical_preprocess_result rejected_source = overlays;
    rejected_source.modules[1].code[0].conditioned_code =
        "int Added( { return 0; }";
    std::vector<asIScriptModule*> untouched{provider};
    standalone::frontend_compile_runtime rejected_runtime;
    const auto rejected = precompiled::compile_mixed_cache_checkpoint(
        *rejected_engine, cache, options, rejected_source,
        nullptr, rejected_runtime, untouched);
    if (rejected.succeeded() ||
        untouched != std::vector<asIScriptModule*>{provider} ||
        rejected_engine->GetModule("Provider", asGM_ONLY_IF_EXISTS) != nullptr ||
        rejected_engine->GetModule("Consumer", asGM_ONLY_IF_EXISTS) != nullptr ||
        rejected_engine->GetModule("Addon", asGM_ONLY_IF_EXISTS) != nullptr) {
        std::cerr << "failed mixed graph was not discarded atomically\n";
        rejected_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 7;
    }

    rejected_engine->ShutDownAndRelease();
    target_engine->ShutDownAndRelease();
    source_engine->ShutDownAndRelease();
    std::cout << "mixed precompiled/source engine smoke passed\n";
    return 0;
}
