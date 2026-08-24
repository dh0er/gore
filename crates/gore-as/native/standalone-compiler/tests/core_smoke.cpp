#include "CoreTypes.h"
#include "angelscript.h"
#include "gore_as_standalone/core.hpp"

#include <cmath>
#include <iostream>

double asStringScanDouble(const char* text);
float asStringScanFloat(const char* text);

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
    TMap<int, int> sparse_map;
    sparse_map.Add(1, 10);
    sparse_map.Add(2, 20);
    sparse_map.Add(3, 30);
    if (sparse_map.Remove(2) != 1) {
        return fail("TMap did not remove the selected sparse slot");
    }
    sparse_map.Add(4, 40);
    const int expected_map_keys[] = {1, 4, 3};
    std::size_t map_index = 0U;
    for (const auto& element : sparse_map) {
        if (map_index >= 3U || element.Key != expected_map_keys[map_index]) {
            return fail("TMap did not preserve UE sparse-slot iteration order");
        }
        ++map_index;
    }
    if (map_index != 3U || sparse_map.FindRef(4) != 40) {
        return fail("TMap sparse-slot reuse lost an entry");
    }

    TMultiMap<int, int> sparse_multimap;
    sparse_multimap.Add(7, 1);
    sparse_multimap.Add(7, 2);
    sparse_multimap.Add(8, 3);
    if (sparse_multimap.Remove(7, 2) != 1) {
        return fail("TMultiMap did not remove the selected sparse slot");
    }
    sparse_multimap.Add(7, 4);
    auto values = sparse_multimap.CreateConstKeyIterator(7);
    if (!values || values.Value() != 4 || !(++values) || values.Value() != 1 || ++values) {
        return fail("TMultiMap key iteration did not use newest-first UE key order");
    }
    TMultiMap<int, int> reused_lower_slot;
    reused_lower_slot.Add(7, 1);
    reused_lower_slot.Add(8, 80);
    reused_lower_slot.Add(7, 2);
    if (reused_lower_slot.Remove(8, 80) != 1) {
        return fail("TMultiMap lower-slot setup did not remove its separator");
    }
    reused_lower_slot.Add(7, 3);
    auto reused_values = reused_lower_slot.CreateConstKeyIterator(7);
    if (!reused_values || reused_values.Value() != 3 ||
        !(++reused_values) || reused_values.Value() != 2 ||
        !(++reused_values) || reused_values.Value() != 1 || ++reused_values) {
        return fail("TMultiMap confused sparse-slot order with key insertion order");
    }
    reused_lower_slot.Add(9, 90); // fourth live element grows and rebuilds UE's hash
    auto rehashed_values = reused_lower_slot.CreateConstKeyIterator(7);
    if (!rehashed_values || rehashed_values.Value() != 2 ||
        !(++rehashed_values) || rehashed_values.Value() != 3 ||
        !(++rehashed_values) || rehashed_values.Value() != 1 || ++rehashed_values) {
        return fail("TMultiMap did not rebuild key order after default-allocator rehash");
    }

    TMultiMap<int, int> reserve_rehash;
    reserve_rehash.Add(7, 1);
    reserve_rehash.Add(8, 80);
    reserve_rehash.Add(7, 2);
    if (reserve_rehash.Remove(8, 80) != 1) {
        return fail("TMultiMap reserve-rehash setup did not remove its separator");
    }
    reserve_rehash.Add(7, 3);
    reserve_rehash.Reserve(4);
    auto reserved_values = reserve_rehash.CreateConstKeyIterator(7);
    if (!reserved_values || reserved_values.Value() != 2 ||
        !(++reserved_values) || reserved_values.Value() != 3 ||
        !(++reserved_values) || reserved_values.Value() != 1 || ++reserved_values) {
        return fail("TMultiMap Reserve did not rebuild the UE hash chain");
    }

    if (std::abs(asStringScanDouble("1.25") - 1.25) > 0.0 ||
        std::abs(asStringScanFloat("1.25") - 1.25F) > 0.0F) {
        return fail("numeric literal scanning did not use the canonical C grammar");
    }

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
