#include "gore_as_standalone/core.hpp"
#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/precompiled_engine.hpp"

#include "as_bytecode.h"

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <map>
#include <string>
#include <unordered_set>
#include <vector>

namespace standalone = gore::as::standalone;
namespace precompiled = gore::as::standalone::precompiled;

namespace {

void message_callback(const asSMessageInfo* message, void*) {
    std::cerr << message->section << ':' << message->row << ':' << message->col
              << ": " << message->message << '\n';
}

std::int32_t skew_function() { return 0; }

std::int32_t static_name_identity(const std::int32_t value) { return value; }

std::int32_t cached_initializer_calls = 0;

void count_cached_initializer(asIScriptGeneric* generic) {
    generic->SetReturnDWord(static_cast<asDWORD>(++cached_initializer_calls));
}

void text_length_generic(asIScriptGeneric* generic) {
    const auto* value = static_cast<const std::string*>(generic->GetArgAddress(0U));
    generic->SetReturnDWord(
        value == nullptr ? 0U : static_cast<asDWORD>(value->size()));
}

class probe_string_factory final : public asIStringFactory {
public:
    const void* GetStringConstant(const char* data, const asUINT length) override {
        auto [iterator, inserted] = values_.try_emplace(std::string(data, length), 0U);
        (void)inserted;
        ++iterator->second;
        return &iterator->first;
    }

    int ReleaseStringConstant(const void* value) override {
        if (value == nullptr) return asINVALID_ARG;
        const auto* text = static_cast<const std::string*>(value);
        const auto iterator = values_.find(*text);
        if (iterator == values_.end() || &iterator->first != text ||
            iterator->second == 0U) {
            return asINVALID_ARG;
        }
        if (--iterator->second == 0U) values_.erase(iterator);
        return asSUCCESS;
    }

    int GetRawStringData(
        const void* value, char* data, asUINT* length) const override {
        if (value == nullptr || length == nullptr) return asINVALID_ARG;
        const auto* text = static_cast<const std::string*>(value);
        const auto iterator = values_.find(*text);
        if (iterator == values_.end() || &iterator->first != text) {
            return asINVALID_ARG;
        }
        if (data != nullptr) std::memcpy(data, text->data(), text->size());
        *length = static_cast<asUINT>(text->size());
        return asSUCCESS;
    }

private:
    std::map<std::string, unsigned int> values_;
};

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
    (void)standalone::compute_processed_code_hash_utf8(
        code, section.code_hash);
    module.code_hash ^= section.code_hash;
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
    consumer->AddPreClassData("LinkedTarget", asPreClassData{});
    consumer->AddPreClassData("LinkedNode", asPreClassData{});
    constexpr char provider_code[] =
        "struct SharedValue { int Number; }\n"
        "int Add(int Left, int Right) { return Left + Right; }";
    constexpr char consumer_code[] =
        "class LinkedTarget { int Value; }\n"
        "class LinkedNode { LinkedTarget Next; }\n"
        "const int InitWitness = CountCachedInitializer();\n"
        "int CallProvider() { return Add(20, 22); }\n"
        "int ReadProviderValue() { SharedValue Value; Value.Number = 42; return Value.Number; }";
    consumer->ImportModule(provider);
    if (source_engine->RegisterGlobalFunction(
            "int CountCachedInitializer()",
            asFUNCTION(count_cached_initializer), asCALL_GENERIC) < 0) {
        source_engine->ShutDownAndRelease();
        return 2;
    }
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
    if (!source_build.succeeded() || cached_initializer_calls != 1 || original_add == nullptr ||
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
    constexpr std::int64_t shadow_pointer = 0x222222222LL;
    precompiled::type_reference shadow_reference;
    shadow_reference.name.bytes = "NativeBase";
    cache.type_references.emplace_back(shadow_pointer, std::move(shadow_reference));
    precompiled::precompiled_module shadow_module;
    shadow_module.module_name.bytes = "Shadow";
    precompiled::precompiled_class shadow_class;
    shadow_class.class_name.bytes = "AShadowChild";
    shadow_class.flags = static_cast<std::int32_t>(
        asOBJ_REF | asOBJ_SCRIPT_OBJECT | asOBJ_NOCOUNT | asOBJ_IMPLICIT_HANDLE);
    shadow_class.shadow_type = shadow_pointer;
    shadow_class.behaviour_references.resize(7U, 0);
    shadow_class.is_in_preprocessor = true;
    shadow_class.super_class.bytes = "NativeBase";
    shadow_class.code_super_class.bytes = "/Script/Test.NativeBase";
    shadow_class.super_is_code_class = true;
    precompiled::precompiled_property shadow_property;
    shadow_property.name.bytes = "Value";
    shadow_property.type = cache.modules[0].second.functions[0].return_type;
    shadow_class.properties.push_back(std::move(shadow_property));
    shadow_module.classes.push_back(std::move(shadow_class));
    cache.modules.emplace_back(module_key("Shadow"), std::move(shadow_module));

    standalone::lexical_preprocess_result overlays;
    overlays.ok = true;
    overlays.modules.push_back(source_module(
        "Provider", "Provider.as",
        "struct SharedValue { int Padding; int Number; }\n"
        "int Add(int Left, int Right) { return Left + Right + 1; }"));
    overlays.modules.push_back(source_module(
        "Addon", "Addon.as",
        "int Added() { return CallProvider(); }", {"Consumer"}));
    overlays.modules.push_back(source_module(
        "Literal", "Literal.as",
        "int LiteralSize() { return ProbeText(\"hello\"); }\n"
        "int Utf8LiteralSize() { return ProbeText(\"Gr\xc3\xbc\xc3\x9f\"); }"));

    asIScriptEngine* const target_engine = asCreateScriptEngine();
    cached_initializer_calls = 0;
    probe_string_factory target_strings;
    target_engine->SetMessageCallback(
        asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    if (target_engine->RegisterGlobalFunction(
            "int Skew()", asFUNCTION(skew_function), asCALL_CDECL) < 0 ||
        target_engine->RegisterGlobalFunction(
            "int CountCachedInitializer()",
            asFUNCTION(count_cached_initializer), asCALL_GENERIC) < 0 ||
        target_engine->RegisterObjectType(
            "NativeBase", 0, asOBJ_REF | asOBJ_NOCOUNT) < 0 ||
        target_engine->RegisterObjectType(
            "Dummy", sizeof(dummy_value),
            asOBJ_VALUE | asOBJ_POD | asGetTypeTraits<dummy_value>()) < 0 ||
        target_engine->RegisterObjectType(
            "Text", sizeof(std::string),
            asOBJ_VALUE | asOBJ_POD | asOBJ_APP_CLASS) < 0 ||
        target_engine->RegisterStringFactory("Text", &target_strings) < 0 ||
        target_engine->RegisterGlobalFunction(
            "int ProbeText(const Text &in Value)",
            asFUNCTION(text_length_generic), asCALL_GENERIC) < 0 ||
        target_engine->RegisterGlobalFunction(
            "int __STATIC_NAME(int Id)",
            asFUNCTION(static_name_identity), asCALL_CDECL) < 0) {
        source_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        return 5;
    }
    standalone::frontend_compile_runtime runtime;
    standalone::preprocessor_options options;
    options.native_super_types.push_back({
        "NativeBase", "/Script/Test.NativeBase", 64U,
        standalone::native_super_kind::other_uobject, false, false});
    std::vector<asIScriptModule*> built;
    const auto mixed = precompiled::compile_mixed_cache_checkpoint(
        *target_engine, cache, options, overlays, nullptr, runtime, built);
    asIScriptModule* const built_provider =
        target_engine->GetModule("Provider", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_consumer =
        target_engine->GetModule("Consumer", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_addon =
        target_engine->GetModule("Addon", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_shadow =
        target_engine->GetModule("Shadow", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const built_literal =
        target_engine->GetModule("Literal", asGM_ONLY_IF_EXISTS);
    asIScriptFunction* const replacement_add = built_provider == nullptr
        ? nullptr
        : built_provider->GetFunctionByName("Add");
    asITypeInfo* const replacement_value = built_provider == nullptr
        ? nullptr
        : built_provider->GetTypeInfoByDecl("SharedValue");
    asITypeInfo* const shadow_type = built_shadow == nullptr
        ? nullptr
        : built_shadow->GetTypeInfoByDecl("AShadowChild");
    int shadow_property_offset = -1;
    if (shadow_type != nullptr) {
        shadow_type->GetProperty(
            0U, nullptr, nullptr, nullptr, nullptr, &shadow_property_offset);
    }
    if (!mixed.succeeded() || cached_initializer_calls != 0 || built.size() != 5U ||
        built[0] != built_provider || built[1] != built_consumer ||
        built[2] != built_shadow || built[3] != built_addon ||
        built[4] != built_literal ||
        replacement_add == nullptr ||
        replacement_value == nullptr ||
        shadow_type == nullptr || shadow_type->GetSize() < 68U ||
        shadow_property_offset != 64 ||
        replacement_add->GetId() == original_add_id ||
        replacement_value->GetTypeId() == original_value_id ||
        !execute_add(*target_engine, *built_provider, 43U) ||
        !execute_no_args(*target_engine, *built_consumer, "CallProvider", 43U) ||
        !execute_no_args(
            *target_engine, *built_consumer, "ReadProviderValue", 42U) ||
        !execute_no_args(*target_engine, *built_addon, "Added", 43U) ||
        !execute_no_args(*target_engine, *built_literal, "LiteralSize", 5U) ||
        !execute_no_args(*target_engine, *built_literal, "Utf8LiteralSize", 6U)) {
        std::cerr << "mixed edit/add graph failed: phase="
                  << static_cast<int>(mixed.phase) << "; module="
                  << mixed.module_index << "; detail=" << mixed.detail << '\n';
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 6;
    }

    const std::array<std::uint8_t, 16U> emitted_guid{
        0x10U, 0x21U, 0x32U, 0x43U, 0x54U, 0x65U, 0x76U, 0x87U,
        0x98U, 0xa9U, 0xbaU, 0xcbU, 0xdcU, 0xedU, 0xfeU, 0x0fU};
    precompiled::cache emitted;
    standalone::registry_runtime export_registry;
    const auto graph_export = precompiled::export_mixed_graph_checkpoint(
        cache, overlays, built, emitted_guid, 4, emitted, export_registry);
    precompiled::codec_error codec;
    std::vector<std::uint8_t> emitted_bytes;
    precompiled::cache decoded;
    const bool encoded = graph_export.succeeded() &&
        precompiled::encode(emitted, emitted_bytes, codec);
    const bool decoded_ok = encoded && precompiled::decode(
        emitted_bytes.data(), emitted_bytes.size(), decoded, codec);
    bool exported_ascii_string = false;
    bool exported_utf8_string = false;
    for (const auto& entry : decoded.global_references) {
        exported_ascii_string = exported_ascii_string ||
            (entry.second.is_string && entry.second.name.bytes == "hello");
        exported_utf8_string = exported_utf8_string ||
            (entry.second.is_string &&
             entry.second.name.bytes == "Gr\xc3\xbc\xc3\x9f");
    }
    if (!decoded_ok || decoded.data_guid != emitted_guid ||
        decoded.build_identifier != 4 || decoded.modules.size() != 5U ||
        decoded.modules[0].second.code_hash != overlays.modules[0].code_hash ||
        decoded.modules[1].second.code_hash !=
            cache.modules[1].second.code_hash ||
        decoded.modules[2].second.classes.size() != 1U ||
        decoded.modules[2].second.classes[0].code_super_class.bytes !=
            "/Script/Test.NativeBase" ||
        decoded.modules[3].second.code_hash != overlays.modules[1].code_hash ||
        decoded.modules[3].second.imported_modules.size() != 1U ||
        decoded.modules[3].second.imported_modules[0].bytes != "Consumer" ||
        decoded.modules[4].second.code_hash != overlays.modules[2].code_hash ||
        !exported_ascii_string || !exported_utf8_string) {
        std::cerr << "mixed graph cache export/wire roundtrip failed: phase="
                  << static_cast<int>(graph_export.phase) << "; module="
                  << graph_export.module_index << "; detail="
                  << graph_export.detail << "; codec=" << codec.field
                  << ':' << codec.detail << '\n';
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 7;
    }

    // FStringInArchive stores new FNames through TCHAR_TO_ANSI. The pinned
    // Windows target replaces each non-ANSI UTF-16 code unit with '?', while
    // preserving already archived base rows byte-for-byte.
    precompiled::cache unicode_base;
    precompiled::archive_string archived_base_name;
    archived_base_name.bytes = "\xc4-base";
    unicode_base.static_names.push_back(std::move(archived_base_name));
    standalone::lexical_preprocess_result unicode_source;
    unicode_source.ok = true;
    unicode_source.static_names = {
        "\xc4-base", "\xc3\x84quivalent", "\xf0\x9f\x98\x80"};
    precompiled::cache unicode_output;
    standalone::registry_runtime unicode_registry;
    const std::vector<asIScriptModule*> no_modules;
    const auto unicode_export = precompiled::export_mixed_graph_checkpoint(
        unicode_base, unicode_source, no_modules, emitted_guid, 4,
        unicode_output, unicode_registry);
    if (!unicode_export.succeeded() || unicode_output.static_names.size() != 3U ||
        unicode_output.static_names[0].bytes != "\xc4-base" ||
        unicode_output.static_names[1].bytes != "?quivalent" ||
        unicode_output.static_names[2].bytes != "??") {
        std::cerr << "target ANSI static-name export failed: "
                  << unicode_export.detail << '\n';
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 8;
    }

    constexpr char qualification_name_code[] = R"AS(
int QualificationNameProjection()
{
    ProbeText('__STATIC_NAME(1)');
    ProbeText("""__STATIC_NAME(1)""");
    return __STATIC_NAME(1) + __STATIC_NAME(2) + __STATIC_NAME(3);
}
)AS";
    constexpr char qualification_local_code[] = R"AS(
int QualificationNameProjection()
{
    ProbeText('__STATIC_NAME(1)');
    ProbeText("""__STATIC_NAME(1)""");
    return __STATIC_NAME(0) + __STATIC_NAME(1) + __STATIC_NAME(2);
}
)AS";
    asIScriptEngine* const qualification_engine = asCreateScriptEngine();
    probe_string_factory qualification_strings;
    if (qualification_engine != nullptr) {
        qualification_engine->SetMessageCallback(
            asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    }
    const bool qualification_environment = qualification_engine != nullptr &&
        qualification_engine->RegisterObjectType(
            "Text", sizeof(std::string),
            asOBJ_VALUE | asOBJ_POD | asOBJ_APP_CLASS) >= 0 &&
        qualification_engine->RegisterStringFactory(
            "Text", &qualification_strings) >= 0 &&
        qualification_engine->RegisterGlobalFunction(
            "int ProbeText(const Text &in Value)",
            asFUNCTION(text_length_generic), asCALL_GENERIC) >= 0 &&
        qualification_engine->RegisterGlobalFunction(
            "int __STATIC_NAME(int Id)",
            asFUNCTION(static_name_identity), asCALL_CDECL) >= 0;
    asIScriptModule* const qualification_name_module = qualification_environment
        ? qualification_engine->GetModule("QualificationNames", asGM_ALWAYS_CREATE)
        : nullptr;
    const bool qualification_name_built = qualification_name_module != nullptr &&
        qualification_name_module->AddScriptSection(
            "QualificationNames.as", qualification_name_code,
            sizeof(qualification_name_code) - 1U) >= 0 &&
        [&]() {
            asIScriptModule* graph[] = {qualification_name_module};
            return standalone::build_module_graph(graph, 1U).succeeded();
        }();
    precompiled::cache qualification_base;
    qualification_base.static_names = {{"Unused"}, {"Existing"}};
    standalone::lexical_preprocess_result qualification_source;
    qualification_source.ok = true;
    qualification_source.static_names = {
        "Unused", "Existing", "\xc3\x84", "\xf0\x9f\x98\x80"};
    qualification_source.static_name_uses = {
        {1U, "Existing"}, {2U, "\xc3\x84"}, {3U, "\xf0\x9f\x98\x80"}};
    qualification_source.modules.push_back(source_module(
        "QualificationNames", "QualificationNames.as", qualification_name_code));
    precompiled::cache qualification_output;
    standalone::registry_runtime qualification_registry;
    const std::vector<asIScriptModule*> qualification_modules{
        qualification_name_module};
    precompiled::engine_bridge_result qualification_export;
    if (qualification_name_built) {
        qualification_export = precompiled::export_source_graph_checkpoint(
            qualification_base, qualification_source, qualification_modules,
            emitted_guid, 4, qualification_output, qualification_registry);
    } else {
        qualification_export.code = asERROR;
        qualification_export.phase = precompiled::engine_bridge_phase::preflight;
        qualification_export.detail = "qualification name module did not compile";
    }
    std::int64_t expected_local_hash = 0;
    const bool expected_hash_ok = standalone::compute_processed_code_hash_utf8(
        qualification_local_code, expected_local_hash);
    std::vector<std::int32_t> projected_name_operands;
    std::unordered_set<std::int64_t> static_name_functions;
    for (const auto& entry : qualification_output.function_references) {
        if (entry.second.name.bytes == "__STATIC_NAME") {
            static_name_functions.insert(entry.first);
        }
    }
    if (!qualification_output.modules.empty()) {
        const auto found = std::find_if(
            qualification_output.modules[0].second.functions.begin(),
            qualification_output.modules[0].second.functions.end(),
            [](const precompiled::precompiled_function& function) {
                return function.function_name.bytes == "QualificationNameProjection";
            });
        if (found != qualification_output.modules[0].second.functions.end()) {
            for (std::size_t offset = 0U; offset < found->byte_code.size();) {
                const auto opcode = static_cast<asEBCInstr>(
                    found->byte_code[offset] & 0xff);
                const std::size_t size = static_cast<std::size_t>(
                    asBCTypeSize[asBCInfo[opcode].type]);
                if (size == 0U || offset > found->byte_code.size() - size) break;
                const std::size_t next = offset + size;
                if (opcode == asBC_PshC4 && size >= 2U &&
                    next < found->byte_code.size()) {
                    const auto next_opcode = static_cast<asEBCInstr>(
                        found->byte_code[next] & 0xff);
                    const std::size_t next_size = static_cast<std::size_t>(
                        asBCTypeSize[asBCInfo[next_opcode].type]);
                    if (next_opcode == asBC_CALLSYS && next_size >= 3U &&
                        next <= found->byte_code.size() - next_size) {
                        const std::uint64_t low = static_cast<std::uint32_t>(
                            found->byte_code[next + 1U]);
                        const std::uint64_t high = static_cast<std::uint32_t>(
                            found->byte_code[next + 2U]);
                        const auto key = static_cast<std::int64_t>(low | (high << 32U));
                        if (static_name_functions.find(key) !=
                            static_name_functions.end()) {
                            projected_name_operands.push_back(
                                found->byte_code[offset + 1U]);
                        }
                    }
                }
                offset = next;
            }
        }
    }
    if (!qualification_export.succeeded() || !expected_hash_ok ||
        qualification_output.static_names.size() != 3U ||
        qualification_output.static_names[0].bytes != "Existing" ||
        qualification_output.static_names[1].bytes != "?" ||
        qualification_output.static_names[2].bytes != "??" ||
        qualification_output.modules.size() != 1U ||
        qualification_output.modules[0].second.code_hash != expected_local_hash ||
        projected_name_operands != std::vector<std::int32_t>{0, 1, 2}) {
        std::cerr << "qualification static-name projection failed: "
                  << qualification_export.detail << "; names=";
        for (const auto& name : qualification_output.static_names) {
            std::cerr << '[' << name.bytes << ']';
        }
        std::cerr << "; hash="
                  << (qualification_output.modules.empty()
                          ? 0
                          : qualification_output.modules[0].second.code_hash)
                  << "; expected_hash=" << expected_local_hash << "; operands=";
        for (const std::int32_t operand : projected_name_operands) {
            std::cerr << operand << ',';
        }
        std::cerr << "; refs=";
        for (const auto& entry : qualification_output.function_references) {
            std::cerr << entry.first << ':' << entry.second.name.bytes << ',';
        }
        std::cerr << "; bytecode=";
        if (!qualification_output.modules.empty() &&
            !qualification_output.modules[0].second.functions.empty()) {
            for (const std::int32_t word :
                 qualification_output.modules[0].second.functions[0].byte_code) {
                std::cerr << word << ',';
            }
        }
        std::cerr << '\n';
        if (qualification_engine != nullptr) {
            qualification_engine->ShutDownAndRelease();
        }
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 9;
    }
    qualification_engine->ShutDownAndRelease();

    standalone::lexical_preprocess_result invalid_export_source = overlays;
    standalone::preprocessed_class_description missing_class;
    missing_class.class_name = "MissingFromCompiledModule";
    invalid_export_source.modules[0].classes.push_back(std::move(missing_class));
    precompiled::cache untouched_export;
    untouched_export.build_identifier = 777;
    precompiled::precompiled_module sentinel_module;
    sentinel_module.module_name.bytes = "Sentinel";
    untouched_export.modules.emplace_back(
        module_key("Sentinel"), std::move(sentinel_module));
    const auto rejected_export = precompiled::export_mixed_graph_checkpoint(
        cache, invalid_export_source, built, emitted_guid, 4, untouched_export,
        export_registry);
    if (rejected_export.succeeded() || untouched_export.build_identifier != 777 ||
        untouched_export.modules.size() != 1U ||
        untouched_export.modules[0].second.module_name.bytes != "Sentinel") {
        std::cerr << "failed mixed graph export changed caller output\n";
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 9;
    }

    asIScriptEngine* const reload_engine = asCreateScriptEngine();
    probe_string_factory reload_strings;
    reload_engine->SetMessageCallback(
        asFUNCTION(message_callback), nullptr, asCALL_CDECL);
    if (reload_engine->RegisterObjectType(
            "NativeBase", 0, asOBJ_REF | asOBJ_NOCOUNT) < 0 ||
        reload_engine->RegisterGlobalFunction(
            "int CountCachedInitializer()",
            asFUNCTION(count_cached_initializer), asCALL_GENERIC) < 0 ||
        reload_engine->RegisterObjectType(
            "Text", sizeof(std::string),
            asOBJ_VALUE | asOBJ_POD | asOBJ_APP_CLASS) < 0 ||
        reload_engine->RegisterStringFactory("Text", &reload_strings) < 0 ||
        reload_engine->RegisterGlobalFunction(
            "int ProbeText(const Text &in Value)",
            asFUNCTION(text_length_generic), asCALL_GENERIC) < 0) {
        reload_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 10;
    }
    standalone::lexical_preprocess_result no_overlays;
    no_overlays.ok = true;
    standalone::frontend_compile_runtime reload_runtime;
    std::vector<asIScriptModule*> reloaded;
    const auto reload = precompiled::compile_mixed_cache_checkpoint(
        *reload_engine, decoded, options, no_overlays,
        nullptr, reload_runtime, reloaded);
    asIScriptModule* const reloaded_provider =
        reload_engine->GetModule("Provider", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const reloaded_consumer =
        reload_engine->GetModule("Consumer", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const reloaded_shadow =
        reload_engine->GetModule("Shadow", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const reloaded_addon =
        reload_engine->GetModule("Addon", asGM_ONLY_IF_EXISTS);
    asIScriptModule* const reloaded_literal =
        reload_engine->GetModule("Literal", asGM_ONLY_IF_EXISTS);
    asITypeInfo* const reloaded_shadow_type = reloaded_shadow == nullptr
        ? nullptr
        : reloaded_shadow->GetTypeInfoByDecl("AShadowChild");
    int reloaded_shadow_offset = -1;
    if (reloaded_shadow_type != nullptr) {
        reloaded_shadow_type->GetProperty(
            0U, nullptr, nullptr, nullptr, nullptr, &reloaded_shadow_offset);
    }
    const bool reload_shape_ok = reload.succeeded() &&
        cached_initializer_calls == 0 && reloaded.size() == 5U &&
        reloaded[0] == reloaded_provider && reloaded[1] == reloaded_consumer &&
        reloaded[2] == reloaded_shadow && reloaded[3] == reloaded_addon &&
        reloaded[4] == reloaded_literal &&
        reloaded_provider != nullptr && reloaded_consumer != nullptr &&
        reloaded_addon != nullptr && reloaded_literal != nullptr &&
        reloaded_shadow_type != nullptr &&
        reloaded_shadow_offset == 64;
    const bool reloaded_add_ok = reload_shape_ok &&
        execute_add(*reload_engine, *reloaded_provider, 43U);
    const bool reloaded_call_ok = reload_shape_ok &&
        execute_no_args(*reload_engine, *reloaded_consumer, "CallProvider", 43U);
    const bool reloaded_property_ok = reload_shape_ok &&
        execute_no_args(
            *reload_engine, *reloaded_consumer, "ReadProviderValue", 42U);
    const bool reloaded_addon_ok = reload_shape_ok &&
        execute_no_args(*reload_engine, *reloaded_addon, "Added", 43U);
    const bool reloaded_literal_ok = reload_shape_ok &&
        execute_no_args(*reload_engine, *reloaded_literal, "LiteralSize", 5U);
    if (!reload_shape_ok ||
        !reloaded_add_ok || !reloaded_call_ok || !reloaded_property_ok ||
        !reloaded_addon_ok || !reloaded_literal_ok) {
        std::cerr << "exported mixed graph did not reload: phase="
                  << static_cast<int>(reload.phase) << "; module="
                  << reload.module_index << "; detail=" << reload.detail << '\n';
        reload_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 10;
    }

    asIScriptEngine* const rejected_engine = asCreateScriptEngine();
    if (rejected_engine->RegisterGlobalFunction(
            "int CountCachedInitializer()",
            asFUNCTION(count_cached_initializer), asCALL_GENERIC) < 0 ||
        rejected_engine->RegisterObjectType(
            "NativeBase", 0, asOBJ_REF | asOBJ_NOCOUNT) < 0) {
        rejected_engine->ShutDownAndRelease();
        reload_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 11;
    }
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
        rejected_engine->GetModule("Shadow", asGM_ONLY_IF_EXISTS) != nullptr ||
        rejected_engine->GetModule("Addon", asGM_ONLY_IF_EXISTS) != nullptr ||
        rejected_engine->GetModule("Literal", asGM_ONLY_IF_EXISTS) != nullptr) {
        std::cerr << "failed mixed graph was not discarded atomically\n";
        rejected_engine->ShutDownAndRelease();
        reload_engine->ShutDownAndRelease();
        target_engine->ShutDownAndRelease();
        source_engine->ShutDownAndRelease();
        return 11;
    }

    rejected_engine->ShutDownAndRelease();
    reload_engine->ShutDownAndRelease();
    target_engine->ShutDownAndRelease();
    source_engine->ShutDownAndRelease();
    std::cout << "mixed precompiled/source engine smoke passed\n";
    return 0;
}
