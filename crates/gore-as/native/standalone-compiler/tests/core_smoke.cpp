#include "angelscript.h"
#include "gore_as_standalone/core.hpp"

#include <iostream>

namespace {

void message_callback(const asSMessageInfo* message, void*) {
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

} // namespace

int main() {
    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) {
        return fail("asCreateScriptEngine returned null");
    }
    if (engine->SetMessageCallback(asFUNCTION(message_callback), nullptr, asCALL_CDECL) < 0) {
        return fail("SetMessageCallback rejected the diagnostic callback", engine);
    }

    asIScriptModule* module = engine->GetModule("generic-smoke", asGM_ALWAYS_CREATE);
    if (module == nullptr) {
        return fail("GetModule returned null", engine);
    }

    constexpr char script[] = R"AS(
        int Fibonacci(int value)
        {
            if (value <= 1)
                return value;
            return Fibonacci(value - 1) + Fibonacci(value - 2);
        }
    )AS";

    if (module->AddScriptSection("generic-smoke.as", script, sizeof(script) - 1U) < 0) {
        return fail("AddScriptSection rejected the smoke source", engine);
    }
    if (gore::as::standalone::build_module(*module) < 0) {
        return fail("AngelScript lexer/parser/builder rejected the smoke source", engine);
    }

    asIScriptFunction* function = module->GetFunctionByName("Fibonacci");
    if (function == nullptr) {
        std::cerr << "module function count: " << module->GetFunctionCount() << '\n';
        for (asUINT index = 0U; index < module->GetFunctionCount(); ++index) {
            const auto* candidate = module->GetFunctionByIndex(index);
            std::cerr << "candidate: " << candidate->GetDeclaration() << '\n';
        }
        return fail("built function was not present in the module", engine);
    }
    asUINT bytecode_length = 0U;
    const asDWORD* bytecode = function->GetByteCode(&bytecode_length);
    if (bytecode == nullptr || bytecode_length == 0U) {
        return fail("built function did not contain bytecode", engine);
    }

    std::cout << "UNREANGEL core smoke built " << bytecode_length << " instructions\n";
    engine->ShutDownAndRelease();
    return 0;
}
