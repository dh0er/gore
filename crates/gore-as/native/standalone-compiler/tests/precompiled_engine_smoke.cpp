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
    asIScriptFunction* const stage3_function = source_module->GetFunctionByName("Add");
    asIScriptFunction* const late_function = source_module->GetFunctionByName("CallAdd");
    gore::as::standalone::shipping_static_jit_candidates static_jit_candidates;
    static_jit_candidates.functions.push_back(stage3_function);
    const std::vector<asIScriptModule*> static_jit_modules{source_module};
    const auto static_jit = precompiled::apply_shipping_static_jit_checkpoint(
        static_jit_modules, static_jit_candidates);
    if (!static_jit.succeeded() || stage3_function == nullptr || late_function == nullptr ||
        !stage3_function->IsFinal() || late_function->IsFinal()) {
        std::cerr << "StaticJIT checkpoint did not preserve the stage-3 candidate boundary\n";
        source_engine->ShutDownAndRelease();
        return 3;
    }
    asIScriptEngine* foreign_engine = asCreateScriptEngine();
    asIScriptModule* foreign_module = foreign_engine == nullptr
        ? nullptr
        : foreign_engine->GetModule("Foreign", asGM_ALWAYS_CREATE);
    constexpr char foreign_source[] = "int ForeignFunction() { return 7; }";
    if (foreign_module == nullptr ||
        foreign_module->AddScriptSection(
            "Foreign.as", foreign_source, sizeof(foreign_source) - 1U) < 0 ||
        gore::as::standalone::build_module(*foreign_module) < 0) {
        std::cerr << "could not build the foreign StaticJIT candidate fixture\n";
        if (foreign_engine != nullptr) foreign_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    gore::as::standalone::shipping_static_jit_candidates foreign_candidates;
    foreign_candidates.functions.push_back(late_function);
    foreign_candidates.functions.push_back(
        foreign_module->GetFunctionByName("ForeignFunction"));
    const auto rejected_static_jit = precompiled::apply_shipping_static_jit_checkpoint(
        static_jit_modules, foreign_candidates);
    if (rejected_static_jit.succeeded() || late_function->IsFinal()) {
        std::cerr << "StaticJIT accepted a foreign candidate or partially mutated the graph\n";
        foreign_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    foreign_engine->ShutDownAndRelease();

    asIScriptEngine* coverage_engine = asCreateScriptEngine();
    asIScriptModule* covered_module = coverage_engine == nullptr
        ? nullptr
        : coverage_engine->GetModule("Covered", asGM_ALWAYS_CREATE);
    asIScriptModule* partial_module = coverage_engine == nullptr
        ? nullptr
        : coverage_engine->GetModule("Partial", asGM_ALWAYS_CREATE);
    constexpr char covered_source[] = "int CoveredFunction() { return 1; }";
    constexpr char partial_source[] =
        "int GeneratedRetained() { return 2; }\n"
        "int ConstructorRetained() { return 3; }\n"
        "int DestructorRetained() { return 4; }\n"
        "int ReflectedRetained() { return 5; }\n"
        "int PlainRetained() { return 6; }\n"
        "int RemovedRetained() { return 7; }\n"
        "int NonFinalFunction() { return 8; }";
    if (covered_module == nullptr || partial_module == nullptr ||
        covered_module->AddScriptSection(
            "Covered.as", covered_source, sizeof(covered_source) - 1U) < 0 ||
        partial_module->AddScriptSection(
            "Partial.as", partial_source, sizeof(partial_source) - 1U) < 0) {
        std::cerr << "could not create the StaticJIT coverage fixture\n";
        if (coverage_engine != nullptr) coverage_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    asIScriptModule* coverage_graph_modules[] = {covered_module, partial_module};
    const auto coverage_build = gore::as::standalone::build_module_graph(
        coverage_graph_modules, 2U);
    asIScriptFunction* const base_generated =
        partial_module->GetFunctionByName("GeneratedRetained");
    asIScriptFunction* const base_constructor =
        partial_module->GetFunctionByName("ConstructorRetained");
    asIScriptFunction* const base_destructor =
        partial_module->GetFunctionByName("DestructorRetained");
    asIScriptFunction* const base_reflected =
        partial_module->GetFunctionByName("ReflectedRetained");
    asIScriptFunction* const base_plain =
        partial_module->GetFunctionByName("PlainRetained");
    asIScriptFunction* const base_removed =
        partial_module->GetFunctionByName("RemovedRetained");
    if (!coverage_build.succeeded() || base_generated == nullptr ||
        base_constructor == nullptr || base_destructor == nullptr ||
        base_reflected == nullptr || base_plain == nullptr || base_removed == nullptr) {
        std::cerr << "could not build the StaticJIT role fixture\n";
        coverage_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    static_cast<asCScriptFunction*>(base_generated)->traits.SetTrait(
        asTRAIT_GENERATED_FUNCTION, true);
    static_cast<asCScriptFunction*>(base_constructor)->traits.SetTrait(
        asTRAIT_CONSTRUCTOR, true);
    static_cast<asCScriptFunction*>(base_destructor)->traits.SetTrait(
        asTRAIT_DESTRUCTOR, true);
    gore::as::standalone::shipping_static_jit_candidates covered_seed;
    covered_seed.functions.push_back(covered_module->GetFunctionByName("CoveredFunction"));
    covered_seed.functions.insert(
        covered_seed.functions.end(),
        {base_generated, base_constructor, base_destructor, base_reflected,
         base_plain, base_removed});
    const std::vector<asIScriptModule*> coverage_modules{
        covered_module, partial_module};
    const auto coverage_seed = precompiled::apply_shipping_static_jit_checkpoint(
        coverage_modules, covered_seed);
    precompiled::shipping_static_jit_coverage coverage;
    const auto derived_coverage =
        precompiled::derive_shipping_static_jit_module_coverage(
            coverage_modules, coverage);
    if (!coverage_seed.succeeded() || !derived_coverage.succeeded() ||
        coverage.base_module_names != std::vector<std::string>({"Covered", "Partial"}) ||
        coverage.fully_analyzed_module_names != std::vector<std::string>{"Covered"} ||
        coverage.retained_final_functions.size() != 6U ||
        !std::all_of(
            coverage.retained_final_functions.begin(),
            coverage.retained_final_functions.end(),
            [](const auto& identity) { return identity.first == "Partial"; }) ||
        !partial_module->GetFunctionByName("NonFinalFunction")->IsFinal()) {
        std::cerr << "StaticJIT coverage was not derived from the sealed trait fixed point\n";
        coverage_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    coverage_engine->ShutDownAndRelease();

    asIScriptEngine* projected_engine = asCreateScriptEngine();
    asIScriptModule* projected_covered = projected_engine == nullptr
        ? nullptr
        : projected_engine->GetModule("Covered", asGM_ALWAYS_CREATE);
    asIScriptModule* projected_partial = projected_engine == nullptr
        ? nullptr
        : projected_engine->GetModule("Partial", asGM_ALWAYS_CREATE);
    asIScriptModule* projected_added = projected_engine == nullptr
        ? nullptr
        : projected_engine->GetModule("Added", asGM_ALWAYS_CREATE);
    constexpr char projected_partial_source[] =
        "int GeneratedRetained() { return 2; }\n"
        "int ConstructorRetained() { return 3; }\n"
        "int DestructorRetained() { return 4; }\n"
        "int ReflectedRetained() { return 5; }\n"
        "int PlainRetained() { return 6; }\n"
        "int NonFinalFunction() { return 8; }";
    constexpr char added_source[] = "int AddedFunction() { return 4; }";
    if (projected_covered == nullptr || projected_partial == nullptr ||
        projected_added == nullptr ||
        projected_covered->AddScriptSection(
            "Covered.as", covered_source, sizeof(covered_source) - 1U) < 0 ||
        projected_partial->AddScriptSection(
            "Partial.as", projected_partial_source,
            sizeof(projected_partial_source) - 1U) < 0 ||
        projected_added->AddScriptSection(
            "Added.as", added_source, sizeof(added_source) - 1U) < 0) {
        std::cerr << "could not create the projected StaticJIT coverage fixture\n";
        if (projected_engine != nullptr) projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    asIScriptModule* projected_graph_modules[] = {
        projected_covered, projected_partial, projected_added};
    const auto projected_build = gore::as::standalone::build_module_graph(
        projected_graph_modules, 3U);
    asIScriptFunction* const projected_generated =
        projected_partial->GetFunctionByName("GeneratedRetained");
    asIScriptFunction* const projected_constructor =
        projected_partial->GetFunctionByName("ConstructorRetained");
    asIScriptFunction* const projected_destructor =
        projected_partial->GetFunctionByName("DestructorRetained");
    asIScriptFunction* const projected_reflected =
        projected_partial->GetFunctionByName("ReflectedRetained");
    asIScriptFunction* const projected_plain =
        projected_partial->GetFunctionByName("PlainRetained");
    asIScriptFunction* const projected_nonfinal =
        projected_partial->GetFunctionByName("NonFinalFunction");
    if (!projected_build.succeeded() || projected_generated == nullptr ||
        projected_constructor == nullptr || projected_destructor == nullptr ||
        projected_reflected == nullptr || projected_plain == nullptr ||
        projected_nonfinal == nullptr) {
        std::cerr << "could not build the projected StaticJIT role fixture\n";
        if (projected_engine != nullptr) projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    static_cast<asCScriptFunction*>(projected_generated)->traits.SetTrait(
        asTRAIT_GENERATED_FUNCTION, true);
    static_cast<asCScriptFunction*>(projected_constructor)->traits.SetTrait(
        asTRAIT_CONSTRUCTOR, true);
    static_cast<asCScriptFunction*>(projected_destructor)->traits.SetTrait(
        asTRAIT_DESTRUCTOR, true);
    gore::as::standalone::shipping_static_jit_candidates projected_candidates;
    for (asIScriptModule* const module : projected_graph_modules) {
        for (asUINT index = 0U; index < module->GetFunctionCount(); ++index) {
            projected_candidates.functions.push_back(module->GetFunctionByIndex(index));
        }
    }
    const std::vector<asIScriptModule*> projected_modules{
        projected_covered, projected_partial, projected_added};

    gore::as::standalone::lexical_preprocess_result projected_source;
    projected_source.ok = true;
    for (const char* const module_name : {"Covered", "Partial", "Added"}) {
        gore::as::standalone::lexical_module_description description;
        description.module_name = module_name;
        projected_source.modules.push_back(std::move(description));
    }
    gore::as::standalone::preprocessed_class_description statics;
    statics.class_name = "Module_PartialStatics";
    statics.is_statics_class = true;
    gore::as::standalone::preprocessed_function_description reflected_description;
    reflected_description.function_name = "ReflectedRetained";
    reflected_description.script_function_name = "ReflectedRetained";
    statics.methods.push_back(std::move(reflected_description));
    projected_source.modules[1].classes.push_back(std::move(statics));

    auto overlapping_coverage = coverage;
    overlapping_coverage.fully_analyzed_module_names.push_back("Partial");
    std::sort(
        overlapping_coverage.fully_analyzed_module_names.begin(),
        overlapping_coverage.fully_analyzed_module_names.end());
    const auto rejected_overlap =
        precompiled::apply_shipping_static_jit_coverage_checkpoint(
            projected_modules, projected_candidates, overlapping_coverage,
            projected_source);
    auto ambiguous_source = projected_source;
    ambiguous_source.modules[1].classes[0].methods.push_back(
        ambiguous_source.modules[1].classes[0].methods[0]);
    const auto rejected_ambiguous =
        precompiled::apply_shipping_static_jit_coverage_checkpoint(
            projected_modules, projected_candidates, coverage, ambiguous_source);
    if (rejected_overlap.succeeded() || rejected_ambiguous.succeeded() ||
        projected_generated->IsFinal() || projected_constructor->IsFinal() ||
        projected_destructor->IsFinal() || projected_reflected->IsFinal() ||
        projected_plain->IsFinal() || projected_nonfinal->IsFinal()) {
        std::cerr << "StaticJIT coverage preflight was ambiguous or partially mutating\n";
        projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }

    asIScriptEngine* const overload_engine = asCreateScriptEngine();
    asIScriptModule* const overload_module = overload_engine == nullptr
        ? nullptr
        : overload_engine->GetModule("OverloadProbe", asGM_ALWAYS_CREATE);
    constexpr char overload_source[] =
        "int Overloaded() { return 9; }\n"
        "int Overloaded(int Value) { return Value; }";
    if (overload_module == nullptr ||
        overload_module->AddScriptSection(
            "OverloadProbe.as", overload_source, sizeof(overload_source) - 1U) < 0 ||
        gore::as::standalone::build_module(*overload_module) < 0) {
        std::cerr << "could not build the StaticJIT overload fixture\n";
        if (overload_engine != nullptr) overload_engine->ShutDownAndRelease();
        projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    const std::vector<asIScriptModule*> overload_modules{overload_module};
    gore::as::standalone::shipping_static_jit_candidates overload_candidates;
    for (asUINT index = 0U; index < overload_module->GetFunctionCount(); ++index) {
        overload_candidates.functions.push_back(
            overload_module->GetFunctionByIndex(index));
    }
    gore::as::standalone::lexical_preprocess_result overload_preprocessing;
    overload_preprocessing.ok = true;
    gore::as::standalone::lexical_module_description overload_description;
    overload_description.module_name = "OverloadProbe";
    gore::as::standalone::preprocessed_class_description overload_statics;
    overload_statics.class_name = "Module_OverloadProbeStatics";
    overload_statics.is_statics_class = true;
    gore::as::standalone::preprocessed_function_description overload_function;
    overload_function.function_name = "Overloaded";
    overload_function.script_function_name = "Overloaded";
    overload_statics.methods.push_back(std::move(overload_function));
    overload_description.classes.push_back(std::move(overload_statics));
    overload_preprocessing.modules.push_back(std::move(overload_description));
    const auto rejected_overload =
        precompiled::apply_shipping_static_jit_coverage_checkpoint(
            overload_modules, overload_candidates, coverage,
            overload_preprocessing);
    if (rejected_overload.succeeded() || projected_generated->IsFinal() ||
        projected_reflected->IsFinal() ||
        overload_module->GetFunctionByIndex(0U)->IsFinal() ||
        overload_module->GetFunctionByIndex(1U)->IsFinal()) {
        std::cerr << "StaticJIT accepted an ambiguous UFUNCTION overload\n";
        overload_engine->ShutDownAndRelease();
        projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    overload_engine->ShutDownAndRelease();
    const auto projected =
        precompiled::apply_shipping_static_jit_coverage_checkpoint(
            projected_modules, projected_candidates, coverage, projected_source);
    if (!projected.succeeded() ||
        !projected_covered->GetFunctionByName("CoveredFunction")->IsFinal() ||
        !projected_generated->IsFinal() || !projected_constructor->IsFinal() ||
        !projected_destructor->IsFinal() || !projected_reflected->IsFinal() ||
        projected_plain->IsFinal() || projected_nonfinal->IsFinal() ||
        !projected_added->GetFunctionByName("AddedFunction")->IsFinal()) {
        std::cerr << "StaticJIT function coverage projection was not exact: "
                  << projected.detail << "; generated=" << projected_generated->IsFinal()
                  << "; constructor=" << projected_constructor->IsFinal()
                  << "; destructor=" << projected_destructor->IsFinal()
                  << "; reflected=" << projected_reflected->IsFinal()
                  << "; plain=" << projected_plain->IsFinal()
                  << "; nonfinal=" << projected_nonfinal->IsFinal() << '\n';
        projected_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 3;
    }
    projected_engine->ShutDownAndRelease();

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
