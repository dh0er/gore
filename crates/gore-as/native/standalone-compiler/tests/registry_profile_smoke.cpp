#include "angelscript.h"
#include "gore_as_standalone/registry_profile.hpp"

#include <cstring>
#include <iostream>
#include <map>
#include <string>

namespace standalone = gore::as::standalone;

namespace {

void inert_global() {}
bool class_template_validator(asITypeInfo* type, void*) {
    return type != nullptr;
}

class probe_string_factory final : public asIStringFactory {
public:
    const void* GetStringConstant(const char* data, asUINT length) override {
        auto [iterator, inserted] = values_.try_emplace(std::string(data, length), 0U);
        (void)inserted;
        ++iterator->second;
        return &iterator->first;
    }
    int ReleaseStringConstant(const void* value) override {
        if (value == nullptr) return asINVALID_ARG;
        const auto* text = static_cast<const std::string*>(value);
        const auto iterator = values_.find(*text);
        if (iterator == values_.end() || &iterator->first != text || iterator->second == 0U) {
            return asINVALID_ARG;
        }
        if (--iterator->second == 0U) values_.erase(iterator);
        return asSUCCESS;
    }
    int GetRawStringData(const void* value, char* data, asUINT* length) const override {
        if (value == nullptr || length == nullptr) return asINVALID_ARG;
        const auto* text = static_cast<const std::string*>(value);
        const auto iterator = values_.find(*text);
        if (iterator == values_.end() || &iterator->first != text) return asINVALID_ARG;
        if (data != nullptr) std::memcpy(data, text->data(), text->size());
        *length = static_cast<asUINT>(text->size());
        return asSUCCESS;
    }
private:
    std::map<std::string, unsigned int> values_;
};

int fail(const std::string& message, asIScriptEngine* engine = nullptr) {
    std::cerr << message << '\n';
    if (engine != nullptr) engine->ShutDownAndRelease();
    return 1;
}

standalone::registration_context context(
    std::string name_space,
    std::optional<std::string> group = std::string("G1R")) {
    standalone::registration_context result;
    result.name_space = std::move(name_space);
    result.config_group = std::move(group);
    result.access_mask = 0xffff'fffeU;
    return result;
}

standalone::registration_entry entry(
    const std::uint32_t ordinal,
    const standalone::registration_kind kind,
    const standalone::registration_context& registration_context) {
    standalone::registration_entry result;
    result.ordinal = ordinal;
    result.registration_id = ordinal;
    result.kind = kind;
    result.context = registration_context;
    return result;
}

void append_result(
    standalone::registry_profile& profile,
    const standalone::registration_result_kind kind,
    const std::uint32_t engine_id = 0U,
    const std::uint32_t owner_engine_type_id = 0U,
    const std::uint32_t index = 0U,
    const bool installed = false) {
    standalone::registration_result result;
    result.ordinal = static_cast<std::uint32_t>(profile.expected_results.size());
    result.trace_registration_id = result.ordinal;
    result.kind = kind;
    result.engine_id = engine_id;
    result.owner_engine_type_id = owner_engine_type_id;
    result.index = index;
    result.installed = installed;
    profile.expected_results.push_back(result);
}

standalone::post_bind_state type_state(
    const std::uint32_t logical_id,
    const std::uint32_t size,
    const std::uint32_t alignment,
    const std::uint32_t flags) {
    standalone::post_bind_state state;
    state.kind = standalone::post_bind_state_kind::object_type;
    state.logical_id = logical_id;
    state.byte_size = size;
    state.alignment = alignment;
    state.flags = flags;
    return state;
}

standalone::post_bind_state function_state(
    const std::uint32_t logical_id,
    const std::uint32_t traits = 0U) {
    standalone::post_bind_state state;
    state.kind = standalone::post_bind_state_kind::function;
    state.logical_id = logical_id;
    state.trait_bits = traits;
    state.exposed_type = 255U;
    return state;
}

standalone::registry_profile make_profile() {
    standalone::registry_profile profile;
    profile.engine_properties = {
        {0U, standalone::engine_property::optimize_bytecode, 1U},
        {1U, standalone::engine_property::use_character_literals, 1U},
        {2U, standalone::engine_property::property_accessor_mode, 3U},
        {3U, standalone::engine_property::allow_implicit_handle_types, 1U},
    };
    profile.host_stubs = {
        {0U, standalone::host_stub_kind::callable, 0U, 1U},
        {1U, standalone::host_stub_kind::callable, 0U, 1U},
        {2U, standalone::host_stub_kind::storage, 8U, 8U},
        {3U, standalone::host_stub_kind::callable, 0U, 1U},
        {4U, standalone::host_stub_kind::object, 0U, 1U},
        {5U, standalone::host_stub_kind::callable, 0U, 1U},
    };

    constexpr std::uint32_t value_flags = asOBJ_VALUE | asOBJ_POD | asOBJ_APP_PRIMITIVE;
    constexpr std::uint32_t text_flags = asOBJ_VALUE | asOBJ_POD | asOBJ_APP_CLASS;
    constexpr std::uint32_t template_flags =
        asOBJ_VALUE | asOBJ_APP_CLASS | asOBJ_TEMPLATE |
        asOBJ_TEMPLATE_SUBTYPE_COVARIANT;
    constexpr std::uint32_t actor_flags = asOBJ_REF | asOBJ_NOCOUNT | asOBJ_IMPLICIT_HANDLE;

    auto value = entry(0U, standalone::registration_kind::object_type, context("Game"));
    value.logical_id = 10U;
    value.declaration = "Value";
    value.byte_size = 8U;
    value.alignment = 8U;
    value.flags = value_flags;
    profile.registrations.push_back(value);

    auto interface_type = entry(1U, standalone::registration_kind::interface_type, context("Game"));
    interface_type.logical_id = 11U;
    interface_type.declaration = "IRunnable";
    profile.registrations.push_back(interface_type);

    auto interface_method = entry(2U, standalone::registration_kind::interface_method, context("Game"));
    interface_method.logical_id = 30U;
    interface_method.owner_type_id = 11U;
    interface_method.declaration = "void Run()";
    profile.registrations.push_back(interface_method);

    auto property = entry(3U, standalone::registration_kind::object_property, context("Game"));
    property.logical_id = 20U;
    property.owner_type_id = 10U;
    property.declaration = "int value";
    property.byte_offset = 4U;
    property.accessor_type = 7U;
    property.is_protected = true;
    profile.registrations.push_back(property);

    auto method = entry(4U, standalone::registration_kind::object_method, context("Game"));
    method.logical_id = 31U;
    method.owner_type_id = 10U;
    method.declaration = "void Run()";
    method.convention = standalone::call_convention::cdecl_object_last;
    method.callable_stub_id = 0U;
    method.accessor_type = 7U;
    profile.registrations.push_back(method);

    auto constructor = entry(5U, standalone::registration_kind::object_behaviour, context("Game"));
    constructor.logical_id = 32U;
    constructor.owner_type_id = 10U;
    constructor.behaviour = standalone::object_behaviour::construct;
    constructor.declaration = "void f()";
    constructor.convention = standalone::call_convention::cdecl_object_last;
    constructor.callable_stub_id = 1U;
    profile.registrations.push_back(constructor);

    auto global_property = entry(6U, standalone::registration_kind::global_property, context(""));
    global_property.logical_id = 21U;
    global_property.declaration = "uint64 Tick";
    global_property.storage_stub_id = 2U;
    profile.registrations.push_back(global_property);

    auto global_function = entry(7U, standalone::registration_kind::global_function, context(""));
    global_function.logical_id = 33U;
    global_function.declaration = "void Log(int value)";
    global_function.convention = standalone::call_convention::cdecl_call;
    global_function.callable_stub_id = 3U;
    profile.registrations.push_back(global_function);

    auto enum_type = entry(8U, standalone::registration_kind::enum_type, context("Game"));
    enum_type.logical_id = 12U;
    enum_type.declaration = "EState";
    profile.registrations.push_back(enum_type);

    auto enum_value = entry(9U, standalone::registration_kind::enum_value, context("Game"));
    enum_value.owner_type_id = 12U;
    enum_value.name = "Ready";
    enum_value.enum_value = 7;
    profile.registrations.push_back(enum_value);

    auto funcdef = entry(10U, standalone::registration_kind::funcdef, context(""));
    funcdef.logical_id = 13U;
    funcdef.declaration = "void Callback(int value)";
    profile.registrations.push_back(funcdef);

    auto typedef_type = entry(11U, standalone::registration_kind::typedef_type, context(""));
    typedef_type.logical_id = 14U;
    typedef_type.name = "Count";
    typedef_type.target_declaration = "uint";
    profile.registrations.push_back(typedef_type);

    auto text = entry(12U, standalone::registration_kind::object_type, context(""));
    text.logical_id = 15U;
    text.declaration = "Text";
    text.byte_size = 8U;
    text.alignment = 8U;
    text.flags = text_flags;
    profile.registrations.push_back(text);

    auto factory = entry(13U, standalone::registration_kind::string_factory, context(""));
    factory.declaration = "Text";
    factory.factory_object_stub_id = 4U;
    profile.registrations.push_back(factory);

    auto template_type = entry(14U, standalone::registration_kind::object_type, context(""));
    template_type.logical_id = 16U;
    template_type.declaration = "TSubclassOf<class T>";
    template_type.byte_size = 8U;
    template_type.alignment = 8U;
    template_type.flags = template_flags;
    profile.registrations.push_back(template_type);

    auto callback = entry(15U, standalone::registration_kind::object_behaviour, context(""));
    callback.logical_id = 34U;
    callback.owner_type_id = 16U;
    callback.behaviour = standalone::object_behaviour::template_callback;
    callback.declaration = "bool f(int&in Type, int&out ErrorMessage)";
    callback.convention = standalone::call_convention::cdecl_call;
    callback.callable_stub_id = 5U;
    callback.validation_adapter = standalone::template_validation_adapter::t_subclass_of;
    profile.registrations.push_back(callback);

    auto default_array = entry(16U, standalone::registration_kind::default_array_type, context(""));
    default_array.declaration = "TSubclassOf<T>";
    profile.registrations.push_back(default_array);

    auto actor = entry(17U, standalone::registration_kind::object_type, context(""));
    actor.logical_id = 17U;
    actor.declaration = "Actor";
    actor.byte_size = 0U;
    actor.alignment = 8U;
    actor.flags = actor_flags;
    profile.registrations.push_back(actor);

    auto second_typedef = entry(18U, standalone::registration_kind::typedef_type, context(""));
    second_typedef.logical_id = 18U;
    second_typedef.name = "Index";
    second_typedef.target_declaration = "uint";
    profile.registrations.push_back(second_typedef);

    auto value_state = type_state(10U, 8U, 8U, value_flags);
    value_state.interface_type_ids = {11U};
    value_state.interface_vft_offsets = {0U};
    value_state.has_implicit_constructors = true;
    profile.final_states.push_back(value_state);

    profile.final_states.push_back(type_state(11U, 0U, 1U, 0U));

    auto object_property_state = standalone::post_bind_state{};
    object_property_state.kind = standalone::post_bind_state_kind::object_property;
    object_property_state.logical_id = 20U;
    object_property_state.byte_offset = 4U;
    object_property_state.access_mask = 0xffff'fffeU;
    object_property_state.is_private = true;
    object_property_state.is_protected = true;
    object_property_state.is_app_bind_property = true;
    object_property_state.exposed_type = 7U;
    profile.final_states.push_back(object_property_state);

    profile.final_states.push_back(function_state(30U));
    profile.final_states.push_back(function_state(31U, 0x200U));
    profile.final_states.push_back(function_state(32U, 0x1U));

    auto global_function_state = function_state(33U, 0x10000U);
    global_function_state.hidden_argument_index = 0U;
    global_function_state.hidden_argument_default = "__WorldContext";
    global_function_state.determines_output_type_argument_index = 0U;
    global_function_state.compile_out = standalone::compile_out_mode::compile_out_entirely;
    global_function_state.first_param = standalone::first_param_metadata::script_function;
    profile.final_states.push_back(global_function_state);

    auto global_state = standalone::post_bind_state{};
    global_state.kind = standalone::post_bind_state_kind::global_property;
    global_state.logical_id = 21U;
    global_state.is_pure_constant = true;
    global_state.pure_constant_value = 42U;
    profile.final_states.push_back(global_state);

    profile.final_states.push_back(type_state(15U, 8U, 8U, text_flags));
    auto template_state = type_state(16U, 8U, 8U, template_flags);
    template_state.accepts_value_subtype = true;
    template_state.accepts_reference_subtype = true;
    profile.final_states.push_back(template_state);
    profile.final_states.push_back(function_state(34U));
    profile.final_states.push_back(type_state(17U, 0U, 8U, actor_flags));
    return profile;
}

bool capture_expected(standalone::registry_profile& profile) {
    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return false;
    probe_string_factory factory;
    std::map<std::uint32_t, int> types;
    if (engine->SetEngineProperty(asEP_OPTIMIZE_BYTECODE, 1U) < 0 ||
        engine->SetEngineProperty(asEP_USE_CHARACTER_LITERALS, 1U) < 0 ||
        engine->SetEngineProperty(asEP_PROPERTY_ACCESSOR_MODE, 3U) < 0 ||
        engine->SetEngineProperty(asEP_ALLOW_IMPLICIT_HANDLE_TYPES, 1U) < 0) {
        engine->ShutDownAndRelease();
        return false;
    }
    const auto set_context = [&](const standalone::registration_entry& item) {
        if (engine->SetDefaultNamespace(item.context.name_space.c_str()) < 0) return false;
        engine->SetDefaultAccessMask(item.context.access_mask);
        return true;
    };
    int code = asERROR;
    for (const standalone::registration_entry& item : profile.registrations) {
        if (!set_context(item)) { engine->ShutDownAndRelease(); return false; }
        const auto owner = [&]() { return types.at(item.owner_type_id); };
        switch (item.kind) {
        case standalone::registration_kind::object_type: {
            const asUINT before = engine->GetObjectTypeCount();
            code = engine->RegisterObjectType(item.declaration.c_str(), item.byte_size, item.flags);
            if (code >= 0) {
                asITypeInfo* type = engine->GetObjectTypeByIndex(before);
                types[item.logical_id] = type->GetTypeId();
                append_result(profile, standalone::registration_result_kind::object_type, type->GetTypeId());
            }
            break;
        }
        case standalone::registration_kind::interface_type: {
            const asUINT before = engine->GetObjectTypeCount();
            code = engine->RegisterInterface(item.declaration.c_str());
            if (code >= 0) {
                asITypeInfo* type = engine->GetObjectTypeByIndex(before);
                types[item.logical_id] = type->GetTypeId();
                append_result(profile, standalone::registration_result_kind::interface_type, type->GetTypeId());
            }
            break;
        }
        case standalone::registration_kind::interface_method:
            code = engine->RegisterInterfaceMethod("IRunnable", item.declaration.c_str());
            if (code >= 0) append_result(profile, standalone::registration_result_kind::interface_method, code, owner());
            break;
        case standalone::registration_kind::object_property: {
            asITypeInfo* type = engine->GetTypeInfoById(owner());
            const asUINT index = type->GetPropertyCount();
            code = engine->RegisterObjectProperty("Value", item.declaration.c_str(), item.byte_offset, 0, false, item.accessor_type, item.is_protected);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::object_property, 0U, owner(), index);
            break;
        }
        case standalone::registration_kind::object_method:
            code = engine->RegisterObjectMethod("Value", item.declaration.c_str(), asFUNCTION(inert_global), asCALL_CDECL_OBJLAST);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::object_method, code, owner());
            break;
        case standalone::registration_kind::object_behaviour:
            if (item.behaviour == standalone::object_behaviour::template_callback) {
                code = engine->RegisterObjectBehaviour("TSubclassOf<T>", asBEHAVE_TEMPLATE_CALLBACK, item.declaration.c_str(), asFUNCTION(class_template_validator), asCALL_CDECL);
            } else {
                code = engine->RegisterObjectBehaviour("Value", asBEHAVE_CONSTRUCT, item.declaration.c_str(), asFUNCTION(inert_global), asCALL_CDECL_OBJLAST);
            }
            if (code >= 0) append_result(profile, standalone::registration_result_kind::object_behaviour, code, owner());
            break;
        case standalone::registration_kind::global_property: {
            static std::uint64_t storage = 0U;
            const asUINT index = engine->GetGlobalPropertyCount();
            code = engine->RegisterGlobalProperty(item.declaration.c_str(), &storage);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::global_property, 0U, 0U, index);
            break;
        }
        case standalone::registration_kind::global_function:
            code = engine->RegisterGlobalFunction(item.declaration.c_str(), asFUNCTION(inert_global), asCALL_CDECL);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::global_function, code);
            break;
        case standalone::registration_kind::enum_type:
            code = engine->RegisterEnum(item.declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                append_result(profile, standalone::registration_result_kind::enum_type, code);
            }
            break;
        case standalone::registration_kind::enum_value: {
            asITypeInfo* type = engine->GetTypeInfoById(owner());
            const asUINT index = type->GetEnumValueCount();
            code = engine->RegisterEnumValue("EState", item.name.c_str(), item.enum_value);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::enum_value, 0U, owner(), index);
            break;
        }
        case standalone::registration_kind::funcdef:
            code = engine->RegisterFuncdef(item.declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                append_result(profile, standalone::registration_result_kind::funcdef, code);
            }
            break;
        case standalone::registration_kind::typedef_type:
            code = engine->RegisterTypedef(item.name.c_str(), item.target_declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                append_result(profile, standalone::registration_result_kind::typedef_type, code);
            }
            break;
        case standalone::registration_kind::string_factory:
            code = engine->RegisterStringFactory(item.declaration.c_str(), &factory);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::string_factory, 0U, 0U, 0U, true);
            break;
        case standalone::registration_kind::default_array_type:
            code = engine->RegisterDefaultArrayType(item.declaration.c_str());
            if (code >= 0) append_result(profile, standalone::registration_result_kind::default_array_type, 0U, 0U, 0U, true);
            break;
        }
        if (code < 0) {
            std::cerr << "probe registration " << item.ordinal << " failed with " << code << '\n';
            engine->ShutDownAndRelease();
            return false;
        }
    }

    for (standalone::post_bind_state& state : profile.final_states) {
        if (state.kind == standalone::post_bind_state_kind::object_type && state.logical_id == 11U) {
            asITypeInfo* interface_type = engine->GetTypeInfoById(types.at(11U));
            state.byte_size = interface_type->GetSize();
            state.alignment = interface_type->alignment;
            state.flags = interface_type->GetFlags();
        }
    }
    engine->ShutDownAndRelease();
    return profile.expected_results.size() == profile.registrations.size();
}

bool rejected_before_mutation(
    const standalone::registry_profile& profile,
    const int expected_code = asINVALID_CONFIGURATION) {
    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return false;
    standalone::registry_runtime runtime;
    const auto result = standalone::replay_registry(*engine, profile, runtime);
    const bool rejected = !result.succeeded() &&
        result.phase == standalone::registry_replay_phase::validate_profile &&
        result.code == expected_code && engine->GetObjectTypeCount() == 0U &&
        engine->GetGlobalFunctionCount() == 0U && engine->GetGlobalPropertyCount() == 0U;
    engine->ShutDownAndRelease();
    return rejected;
}

} // namespace

int main() {
    standalone::registry_profile profile = make_profile();
    if (!capture_expected(profile)) return fail("could not capture deterministic synthetic identities");

    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return fail("asCreateScriptEngine returned null");
    standalone::registry_runtime runtime;
    const standalone::registry_replay_result replay =
        standalone::replay_registry(*engine, profile, runtime);
    if (!replay.succeeded()) {
        return fail("registry replay failed at phase " + std::to_string(static_cast<int>(replay.phase)) +
            " ordinal " + std::to_string(replay.failed_ordinal) + ": " + replay.detail, engine);
    }
    const auto property_mode = engine->GetEngineProperty(asEP_PROPERTY_ACCESSOR_MODE);
    const int value_type = engine->GetTypeIdByDecl("Game::Value");
    const int valid_template = engine->GetTypeIdByDecl("TSubclassOf<Actor>");
    const int invalid_template = engine->GetTypeIdByDecl("TSubclassOf<int>");
    const int default_array = engine->GetDefaultArrayTypeId();
    const int string_type = engine->GetStringFactoryReturnTypeId();
    if (property_mode != 3U || value_type < 0 || valid_template < 0 ||
        invalid_template >= 0 || default_array < 0 || string_type < 0) {
        std::cerr << "property=" << property_mode << " value=" << value_type
                  << " valid=" << valid_template << " invalid=" << invalid_template
                  << " array=" << default_array << " string=" << string_type << '\n';
        return fail("replayed registry is not usable or template validation changed", engine);
    }

    asIScriptEngine* reused_runtime_engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (reused_runtime_engine == nullptr) {
        return fail("runtime-reuse asCreateScriptEngine returned null", engine);
    }
    const auto runtime_reuse =
        standalone::replay_registry(*reused_runtime_engine, profile, runtime);
    if (runtime_reuse.succeeded() ||
        runtime_reuse.phase != standalone::registry_replay_phase::validate_profile ||
        reused_runtime_engine->GetObjectTypeCount() != 0U) {
        reused_runtime_engine->ShutDownAndRelease();
        return fail("bound registry runtime was reused", engine);
    }
    reused_runtime_engine->ShutDownAndRelease();

    standalone::registry_profile wrong = profile;
    wrong.expected_results[4].engine_id += 100'000U;
    asIScriptEngine* rejected_engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (rejected_engine == nullptr) return fail("second asCreateScriptEngine returned null", engine);
    standalone::registry_runtime rejected_runtime;
    const auto rejected = standalone::replay_registry(*rejected_engine, wrong, rejected_runtime);
    if (rejected.succeeded() || rejected.phase != standalone::registry_replay_phase::verify_registration_result ||
        rejected.failed_ordinal != 4U) {
        rejected_engine->ShutDownAndRelease();
        return fail("captured registration identity mismatch did not fail closed", engine);
    }
    rejected_engine->ShutDownAndRelease();

    standalone::registry_profile invalid_owner = profile;
    invalid_owner.registrations[3].owner_type_id = 11U;
    if (!rejected_before_mutation(invalid_owner)) {
        return fail("non-object property owner did not fail before mutation", engine);
    }

    standalone::registry_profile unreferenced_stub = profile;
    unreferenced_stub.host_stubs.push_back(
        {6U, standalone::host_stub_kind::callable, 0U, 1U});
    if (!rejected_before_mutation(unreferenced_stub)) {
        return fail("unreferenced host stub did not fail before mutation", engine);
    }

    standalone::registry_profile incomplete_snapshot = profile;
    incomplete_snapshot.final_states.pop_back();
    if (!rejected_before_mutation(incomplete_snapshot)) {
        return fail("incomplete final-state snapshot did not fail before mutation", engine);
    }

    standalone::registry_profile unsupported = profile;
    unsupported.registrations[15].validation_adapter =
        standalone::template_validation_adapter::t_array;
    if (!rejected_before_mutation(unsupported, asNOT_SUPPORTED)) {
        return fail("unported container validator did not reject before mutation", engine);
    }

    std::cout << "registry profile replayed every registration class, post-bind state, "
                 "string storage, closed class-template validation, and fail-closed preflight\n";
    engine->ShutDownAndRelease();
    return 0;
}
