#include "angelscript.h"
#include "gore_as_standalone/core.hpp"

#include <iostream>

namespace {

struct diagnostic_state {
    bool expect_errors = false;
    unsigned int expected_error_count = 0U;
};

void message_callback(const asSMessageInfo* message, void* user_data) {
    auto& state = *static_cast<diagnostic_state*>(user_data);
    if (message->type == asMSGTYPE_ERROR && state.expect_errors) {
        ++state.expected_error_count;
        return;
    }
    std::cerr << message->section << ':' << message->row << ':' << message->col
              << ": " << message->message << '\n';
}

int fail(const char* message, asIScriptEngine* engine = nullptr) {
    std::cerr << message << '\n';
    if (engine != nullptr) {
        engine->ShutDownAndRelease();
    }
    return 1;
}

bool contains_bytecode(asIScriptModule& module, const char* function_name) {
    asIScriptFunction* function = module.GetFunctionByName(function_name);
    if (function == nullptr) {
        return false;
    }
    asUINT bytecode_length = 0U;
    return function->GetByteCode(&bytecode_length) != nullptr && bytecode_length != 0U;
}

} // namespace

int main() {
    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) {
        return fail("asCreateScriptEngine returned null");
    }

    diagnostic_state diagnostics{};
    if (engine->SetMessageCallback(
            asFUNCTION(message_callback), &diagnostics, asCALL_CDECL) < 0) {
        return fail("SetMessageCallback rejected the diagnostic callback", engine);
    }

    asIScriptModule* provider = engine->GetModule("provider", asGM_ALWAYS_CREATE);
    asIScriptModule* consumer = engine->GetModule("consumer", asGM_ALWAYS_CREATE);
    if (provider == nullptr || consumer == nullptr) {
        return fail("GetModule returned null", engine);
    }

    constexpr char provider_source[] = R"AS(
        enum ProviderKind
        {
            Ready
        }
    )AS";
    constexpr char consumer_source[] = R"AS(
        ProviderKind PassThrough(ProviderKind value)
        {
            return value;
        }
    )AS";

    if (provider->AddScriptSection(
            "provider.as", provider_source, sizeof(provider_source) - 1U) < 0 ||
        consumer->AddScriptSection(
            "consumer.as", consumer_source, sizeof(consumer_source) - 1U) < 0) {
        return fail("AddScriptSection rejected a graph source", engine);
    }

    // The consumer intentionally precedes the provider. Its declaration can
    // only resolve ProviderKind if type generation is a graph-wide barrier.
    consumer->ImportModule(provider);
    asIScriptModule* graph[] = {consumer, provider};
    const auto graph_result = gore::as::standalone::build_module_graph(graph, 2U);
    if (!graph_result.succeeded()) {
        return fail("graph-wide build rejected a cross-module type declaration", engine);
    }
    if (provider->GetTypeInfoByName("ProviderKind") == nullptr ||
        !contains_bytecode(*consumer, "PassThrough")) {
        return fail("cross-module graph output was incomplete", engine);
    }

    // A failed graph must release both the engine build lock and every source
    // builder, leaving the module reusable for a later clean build.
    asIScriptModule* recovery = engine->GetModule("recovery", asGM_ALWAYS_CREATE);
    if (recovery == nullptr ||
        recovery->AddScriptSection("broken.as", "int Broken( {", 13U) < 0) {
        return fail("could not prepare the recovery probe", engine);
    }
    diagnostics.expect_errors = true;
    asIScriptModule* failing_graph[] = {recovery};
    const auto failed_result =
        gore::as::standalone::build_module_graph(failing_graph, 1U);
    diagnostics.expect_errors = false;
    if (failed_result.succeeded() ||
        failed_result.phase != gore::as::standalone::graph_build_phase::parse_scripts ||
        failed_result.failed_module != 0U ||
        diagnostics.expected_error_count == 0U) {
        return fail("malformed graph did not fail at the parse barrier", engine);
    }

    constexpr char recovered_source[] = "int Recovered() { return 42; }";
    if (recovery->AddScriptSection(
            "recovered.as", recovered_source, sizeof(recovered_source) - 1U) < 0) {
        return fail("failed graph did not release its builder", engine);
    }
    const auto recovered_result =
        gore::as::standalone::build_module_graph(failing_graph, 1U);
    if (!recovered_result.succeeded() || !contains_bytecode(*recovery, "Recovered")) {
        return fail("failed graph did not release the engine build session", engine);
    }

    std::cout << "UNREANGEL graph smoke built a consumer-before-provider dependency"
              << " and recovered after a parse failure\n";
    engine->ShutDownAndRelease();
    return 0;
}
