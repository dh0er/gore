#include "gore_as_standalone/core.hpp"
#include "gore_as_standalone/precompiled_data.hpp"
#include "gore_as_standalone/precompiled_engine.hpp"

#include "as_scriptfunction.h"

#include <algorithm>
#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

namespace precompiled = gore::as::standalone::precompiled;

namespace {

struct counter_value {
    std::int32_t health = 0;
};

struct dummy_value {
    std::int32_t value = 0;
};

void message_callback(const asSMessageInfo* message, void*) {
    std::cerr << message->section << ':' << message->row << ':' << message->col
              << ": " << message->message << '\n';
}

void accept_generic_value(asIScriptGeneric*) {}

precompiled::map_string module_key(const char* const name) {
    const std::string text(name);
    return precompiled::map_string{
        false, std::vector<std::uint8_t>(text.begin(), text.end())};
}

bool execute_function(
    asIScriptEngine& engine,
    asIScriptModule& module,
    const char* const name,
    const bool has_arguments) {
    asIScriptFunction* function = module.GetFunctionByName(name);
    if (function == nullptr) {
        std::cerr << "rehydrated function was not found: " << name << '\n';
        return false;
    }
    asIScriptContext* context = engine.CreateContext();
    const bool prepared = context != nullptr && context->Prepare(function) >= 0;
    const bool arguments_set = !has_arguments ||
        (context != nullptr && context->SetArgDWord(0U, 20U) >= 0 &&
         context->SetArgDWord(1U, 22U) >= 0);
    const int execution = prepared && arguments_set ? context->Execute() : asERROR;
    if (!prepared || !arguments_set || execution != asEXECUTION_FINISHED ||
        context->GetReturnDWord() != 42U) {
        std::cerr << "rehydrated execution did not return 42: " << name << '\n';
        std::cerr << "execution=" << execution
                  << "; return=" << (context == nullptr ? 0U : context->GetReturnDWord())
                  << "; exception="
                  << (context == nullptr || context->GetExceptionString() == nullptr
                          ? "<none>"
                          : context->GetExceptionString()) << '\n';
        if (context != nullptr) {
            context->Release();
        }
        return false;
    }
    context->Release();
    return true;
}

bool validate_ref_type(
    asIScriptModule& module,
    const char* const type_name,
    const char* const method_name,
    const char* const property_name) {
    asITypeInfo* type = module.GetTypeInfoByDecl(type_name);
    asIScriptFunction* method =
        type == nullptr ? nullptr : type->GetMethodByName(method_name);
    int property_offset = -1;
    const char* actual_property_name = nullptr;
    if (type == nullptr || method == nullptr || type->GetPropertyCount() != 1U ||
        type->GetProperty(
            0U, &actual_property_name, nullptr, nullptr, nullptr,
            &property_offset) < 0 ||
        actual_property_name == nullptr ||
        std::string(actual_property_name) != property_name || property_offset < 0 ||
        static_cast<asUINT>(property_offset + sizeof(std::int32_t)) > type->GetSize()) {
        std::cerr << "script reference type/method/property was not reconstructed\n";
        return false;
    }
    return (type->GetFlags() & (asOBJ_REF | asOBJ_SCRIPT_OBJECT)) ==
               (asOBJ_REF | asOBJ_SCRIPT_OBJECT) &&
           method->GetParamCount() == 0U;
}

bool compile_class_consumer(asIScriptEngine& engine, asIScriptModule& provider) {
    asITypeInfo* source_type = provider.GetTypeInfoByDecl("UCounter");
    if (source_type == nullptr || source_type->GetPropertyCount() != 1U) {
        std::cerr << "rehydrated UCounter type/property was not found\n";
        return false;
    }
    asIScriptModule* consumer = engine.GetModule("Consumer", asGM_ALWAYS_CREATE);
    constexpr char source[] = R"AS(
        int UseCounter()
        {
            UCounter Value;
            Value.Health = 40;
            return Value.Read();
        }
        int SumDerived(DerivedCounter Value)
        {
            return Value.BaseHealth + Value.Bonus;
        }
    )AS";
    consumer->ImportModule(&provider);
    const int added = consumer->AddScriptSection("Consumer.as", source, sizeof(source) - 1U);
    const int built = added >= 0 ? gore::as::standalone::build_module(*consumer) : added;
    if (added < 0 || built < 0 || consumer->GetFunctionByName("SumDerived") == nullptr ||
        !execute_function(engine, *consumer, "UseCounter", false)) {
        std::cerr << "consumer could not compile against rehydrated class/property\n";
        return false;
    }
    return true;
}

bool register_counter(
    asIScriptEngine& engine,
    counter_value& counter,
    const bool add_target_id_skew) {
    if (add_target_id_skew &&
        engine.RegisterObjectType(
            "Dummy", sizeof(dummy_value),
            asOBJ_VALUE | asOBJ_POD | asGetTypeTraits<dummy_value>()) < 0) {
        return false;
    }
    return engine.RegisterObjectType(
               "Counter", sizeof(counter_value),
               asOBJ_VALUE | asOBJ_POD | asGetTypeTraits<counter_value>()) >= 0 &&
           engine.RegisterObjectProperty(
               "Counter", "int Health", asOFFSET(counter_value, health)) >= 0 &&
           engine.RegisterGlobalProperty("Counter CounterValue", &counter) >= 0;
}

} // namespace

int main() {
    asIScriptEngine* source_engine = asCreateScriptEngine();
    if (source_engine == nullptr) {
        std::cerr << "could not create source engine\n";
        return 1;
    }
    source_engine->SetMessageCallback(asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    std::int32_t source_seed = 40;
    counter_value source_counter{40};
    if (source_engine->RegisterGlobalProperty("int Seed", &source_seed) < 0 ||
        source_engine->RegisterGlobalFunction(
            "void AcceptGeneric(const ?&in Value)",
            asFUNCTION(accept_generic_value), asCALL_GENERIC) < 0 ||
        !register_counter(*source_engine, source_counter, false)) {
        std::cerr << "could not register source globals/types\n";
        source_engine->ShutDownAndRelease();
        return 2;
    }
    asIScriptModule* source_module = source_engine->GetModule("RoundTrip", asGM_ALWAYS_CREATE);
    constexpr char source[] = R"AS(
        int Add(int Left, int Right) { return Left + Right; }
        int CallAdd() { return Add(20, 22); }
        int ReadSeed() { return Seed + 2; }
        void CallGeneric() { AcceptGeneric(Seed); }
        int ReadCounter() { return CounterValue.Health + 2; }
        struct UCounter
        {
            int Health;
            int Read() { return Health + 2; }
        }
        class RefCounter
        {
            int Health;
            int Read() const { return Health + 2; }
        }
    )AS";
    if (source_module == nullptr) {
        std::cerr << "source module was not created\n";
        source_engine->ShutDownAndRelease();
        return 2;
    }
    source_module->AddPreClassData("RefCounter", asPreClassData{});
    if (
        source_module->AddScriptSection("RoundTrip.as", source, sizeof(source) - 1U) < 0 ||
        gore::as::standalone::build_module(*source_module) < 0 ||
        !validate_ref_type(*source_module, "RefCounter", "Read", "Health")) {
        std::cerr << "source module failed to compile\n";
        source_engine->ShutDownAndRelease();
        return 2;
    }
    precompiled::cache cache;
    cache.build_identifier = static_cast<std::int32_t>(0x9e377abeU);
    precompiled::precompiled_module exported;
    const precompiled::map_string key = module_key("RoundTrip");
    const auto export_result = precompiled::export_module_checkpoint(
        *source_module, key, precompiled::archive_string{"RoundTrip.as"},
        0x1234, exported, &cache);
    if (!export_result.succeeded()) {
        std::cerr << "module export failed: " << export_result.detail << '\n';
        source_engine->ShutDownAndRelease();
        return 3;
    }

    const auto exported_add = std::find_if(
        exported.functions.begin(), exported.functions.end(),
        [](const precompiled::precompiled_function& function) {
            return function.function_name.bytes == "Add";
        });
    if (exported_add == exported.functions.end()) {
        std::cerr << "exported Add function was not found\n";
        source_engine->ShutDownAndRelease();
        return 4;
    }
    const precompiled::data_type int_type = exported_add->return_type;
    precompiled::precompiled_class base_class;
    base_class.class_name.bytes = "BaseCounter";
    base_class.flags = static_cast<std::int32_t>(
        asOBJ_REF | asOBJ_SCRIPT_OBJECT | asOBJ_NOCOUNT | asOBJ_IMPLICIT_HANDLE);
    base_class.behaviour_references.resize(7U, 0);
    precompiled::precompiled_property base_property;
    base_property.name.bytes = "BaseHealth";
    base_property.type = int_type;
    base_class.properties.push_back(std::move(base_property));
    exported.classes.push_back(std::move(base_class));

    constexpr std::int64_t base_type_pointer = 0x111111111LL;
    precompiled::type_reference base_reference;
    base_reference.name.bytes = "BaseCounter";
    base_reference.module.bytes = "RoundTrip";
    cache.type_references.emplace_back(base_type_pointer, std::move(base_reference));
    precompiled::precompiled_class derived_class;
    derived_class.class_name.bytes = "DerivedCounter";
    derived_class.flags = static_cast<std::int32_t>(
        asOBJ_REF | asOBJ_SCRIPT_OBJECT | asOBJ_NOCOUNT | asOBJ_IMPLICIT_HANDLE);
    derived_class.derived_from = base_type_pointer;
    derived_class.behaviour_references.resize(7U, 0);
    precompiled::precompiled_property derived_property;
    derived_property.name.bytes = "Bonus";
    derived_property.type = int_type;
    derived_class.properties.push_back(std::move(derived_property));
    exported.classes.push_back(std::move(derived_class));

    if (cache.function_references.empty() ||
        cache.function_id_reference_to_pointer.empty() ||
        cache.global_references.empty() || cache.type_references.empty() ||
        cache.type_id_reference_to_pointer.empty() || cache.property_references.empty()) {
        std::cerr << "reference exporter did not populate all required tails\n";
        source_engine->ShutDownAndRelease();
        return 4;
    }
    cache.modules.emplace_back(key, std::move(exported));
    precompiled::precompiled_module import_probe;
    import_probe.module_name.bytes = "Import Probe";
    import_probe.script_relative_filename.bytes = "ImportProbe.as";
    import_probe.imported_modules.push_back(precompiled::archive_string{"RoundTrip"});
    precompiled::function_import imported_add;
    imported_add.imported_from_module.bytes = "RoundTrip";
    const auto add_record = std::find_if(
        cache.modules.front().second.functions.begin(),
        cache.modules.front().second.functions.end(),
        [](const precompiled::precompiled_function& function) {
            return function.function_name.bytes == "Add";
        });
    if (add_record == cache.modules.front().second.functions.end()) {
        std::cerr << "exported Add function was not found\n";
        source_engine->ShutDownAndRelease();
        return 5;
    }
    imported_add.signature.name = add_record->function_name;
    imported_add.signature.name_space = add_record->name_space;
    imported_add.signature.return_type = add_record->return_type;
    imported_add.signature.parameter_types = add_record->parameter_types;
    imported_add.signature.parameter_flags = add_record->parameter_flags;
    imported_add.signature.parameter_default_args = add_record->parameter_default_args;
    import_probe.function_imports.push_back(std::move(imported_add));
    cache.modules.emplace_back(module_key("Import Probe"), std::move(import_probe));
    precompiled::codec_error codec_error;
    std::vector<std::uint8_t> bytes;
    if (!precompiled::encode(cache, bytes, codec_error)) {
        std::cerr << "cache encode failed: " << codec_error.detail << '\n';
        source_engine->ShutDownAndRelease();
        return 5;
    }
    precompiled::cache decoded;
    if (!precompiled::decode(bytes.data(), bytes.size(), decoded, codec_error)) {
        std::cerr << "cache decode failed: " << codec_error.detail << '\n';
        source_engine->ShutDownAndRelease();
        return 6;
    }

    asIScriptEngine* target_engine = asCreateScriptEngine();
    target_engine->SetMessageCallback(asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    std::int32_t target_seed = 40;
    counter_value target_counter{40};
    if (target_engine->RegisterGlobalProperty("int Seed", &target_seed) < 0 ||
        target_engine->RegisterGlobalFunction(
            "void AcceptGeneric(const ?&in Value)",
            asFUNCTION(accept_generic_value), asCALL_GENERIC) < 0 ||
        !register_counter(*target_engine, target_counter, true)) {
        std::cerr << "could not register target globals/types\n";
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 7;
    }
    if (target_engine->SetDefaultNamespace("LeakedRegistrationContext") < 0) {
        std::cerr << "could not stage non-root registration context\n";
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 7;
    }
    std::vector<asIScriptModule*> loaded;
    const auto load_result =
        precompiled::rehydrate_cache_checkpoint(*target_engine, decoded, loaded);
    target_engine->SetDefaultNamespace("");
    const asUINT imported_count = loaded.size() == 2U
        ? loaded[1]->GetImportedFunctionCount()
        : 0U;
    asIScriptFunction* provider_add = loaded.empty()
        ? nullptr
        : loaded[0]->GetFunctionByName("Add");
    const int bind_result = imported_count == 1U && provider_add != nullptr
        ? loaded[1]->BindImportedFunction(0U, provider_add)
        : asERROR;
    if (!load_result.succeeded() || loaded.size() != 2U ||
        imported_count != 1U || bind_result < 0 ||
        !execute_function(*target_engine, *loaded[0], "Add", true) ||
        !execute_function(*target_engine, *loaded[0], "CallAdd", false) ||
        !execute_function(*target_engine, *loaded[0], "ReadSeed", false) ||
        !execute_function(*target_engine, *loaded[0], "ReadCounter", false) ||
        !validate_ref_type(*loaded[0], "RefCounter", "Read", "Health") ||
        !compile_class_consumer(*target_engine, *loaded[0])) {
        std::cerr << "cache rehydration failed: " << load_result.detail
                  << "; modules=" << loaded.size()
                  << "; imports=" << imported_count
                  << "; bind=" << bind_result << '\n';
        if (loaded.size() == 2U && imported_count == 1U) {
            std::cerr << "imported declaration: "
                      << loaded[1]->GetImportedFunctionDeclaration(0U) << '\n';
            std::cerr << "provider declaration: "
                      << (provider_add == nullptr
                              ? "<missing>"
                              : provider_add->GetDeclaration()) << '\n';
        }
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 7;
    }

    // The bridge must reject full function-state corruption during preflight,
    // before any module becomes visible in the target engine.
    std::vector<std::pair<const char*, precompiled::cache>> invalid_function_caches;
    {
        precompiled::cache invalid = decoded;
        auto& function = invalid.modules.front().second.functions.front();
        function.variable_space = 1;
        function.stack_needed = 0;
        invalid_function_caches.emplace_back("stack below variable space", std::move(invalid));
    }
    {
        precompiled::cache invalid = decoded;
        auto& function = invalid.modules.front().second.functions.front();
        function.object_variables_on_heap =
            static_cast<std::int32_t>(function.object_variable_positions.size() + 1U);
        invalid_function_caches.emplace_back("invalid object heap prefix", std::move(invalid));
    }
    {
        precompiled::cache invalid = decoded;
        auto& function = invalid.modules.front().second.functions.front();
        function.variable_info_program_positions.push_back(0);
        function.variable_info_offsets.push_back(0);
        function.variable_info_options.push_back(asBLOCK_BEGIN);
        invalid_function_caches.emplace_back("unclosed variable-info block", std::move(invalid));
    }
    {
        precompiled::cache invalid = decoded;
        invalid.modules.front().second.functions.front().line_numbers.push_back(0);
        invalid_function_caches.emplace_back("odd line-number array", std::move(invalid));
    }
    {
        precompiled::cache invalid = decoded;
        auto& functions = invalid.modules.front().second.functions;
        functions[1].id = functions[0].id;
        invalid_function_caches.emplace_back("duplicate function id", std::move(invalid));
    }
    for (const auto& [label, invalid] : invalid_function_caches) {
        asIScriptEngine* invalid_engine = asCreateScriptEngine();
        std::vector<asIScriptModule*> invalid_loaded{source_module};
        const auto invalid_result =
            precompiled::rehydrate_cache_checkpoint(*invalid_engine, invalid, invalid_loaded);
        if (invalid_result.succeeded() ||
            invalid_loaded != std::vector<asIScriptModule*>{source_module} ||
            invalid_engine->GetModule("RoundTrip", asGM_ONLY_IF_EXISTS) != nullptr) {
            std::cerr << "invalid function state was not rejected atomically: " << label << '\n';
            invalid_engine->ShutDownAndRelease();
            target_engine->ShutDownAndRelease();
            source_engine->ShutDownAndRelease();
            return 8;
        }
        invalid_engine->ShutDownAndRelease();
    }

    // Unsupported reference tables must reject the whole cache before module
    // creation, leaving both the caller output and target engine untouched.
    asIScriptEngine* rejected_engine = asCreateScriptEngine();
    precompiled::cache unsupported = decoded;
    unsupported.type_id_reference_to_pointer.emplace_back(1, 2);
    std::vector<asIScriptModule*> untouched{source_module};
    const auto rejected =
        precompiled::rehydrate_cache_checkpoint(*rejected_engine, unsupported, untouched);
    if (rejected.succeeded() || untouched != std::vector<asIScriptModule*>{source_module} ||
        rejected_engine->GetModule("RoundTrip", asGM_ONLY_IF_EXISTS) != nullptr) {
        std::cerr << "unsupported cache was not rejected atomically\n";
        rejected_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 9;
    }

    rejected_engine->ShutDownAndRelease();
    target_engine->ShutDownAndRelease();
    source_engine->ShutDownAndRelease();
    std::cout << "precompiled engine export/rehydration smoke passed\n";
    return 0;
}
