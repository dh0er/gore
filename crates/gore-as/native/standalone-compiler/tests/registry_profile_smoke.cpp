#include "angelscript.h"
#include "as_callfunc.h"
#include "as_scriptfunction.h"
#include "gore_as_standalone/core.hpp"
#include "gore_as_standalone/registry_profile.hpp"

#include <algorithm>
#include <cstring>
#include <iostream>
#include <map>
#include <string>
#include <string_view>
#include <vector>

namespace standalone = gore::as::standalone;

namespace {

void inert_global() {}
struct inert_object {
    void method() {}
};
struct message_log {
    std::vector<std::string> messages;

    static void receive(const asSMessageInfo* message, void* parameter) {
        if (message == nullptr || parameter == nullptr || message->message == nullptr) return;
        static_cast<message_log*>(parameter)->messages.emplace_back(message->message);
    }

    bool contains(const std::string_view text) const {
        for (const std::string& message : messages) {
            if (message.find(text) != std::string::npos) return true;
        }
        return false;
    }
};

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

standalone::fixed_type_operations pod_operations(
    const std::uint32_t size,
    const std::uint32_t alignment) {
    standalone::fixed_type_operations operations;
    operations.can_create_property = true;
    operations.can_be_template_subtype = true;
    operations.can_construct = true;
    operations.need_construct = false;
    operations.can_destruct = true;
    operations.need_destruct = false;
    operations.can_copy = true;
    operations.need_copy = false;
    operations.can_compare = true;
    operations.can_hash_value = true;
    operations.value_size = size;
    operations.value_alignment = alignment;
    return operations;
}

standalone::type_operations fixed_operations(
    const std::uint32_t size,
    const std::uint32_t alignment) {
    standalone::type_operations operations;
    operations.kind = standalone::type_operations_kind::fixed;
    operations.fixed = pod_operations(size, alignment);
    return operations;
}

standalone::type_operations container_operations(
    const standalone::type_operations_kind kind) {
    standalone::type_operations operations;
    operations.kind = kind;
    return operations;
}

std::string callable_declaration(std::string declaration) {
    constexpr std::string_view marker = "class ";
    std::size_t position = 0U;
    while ((position = declaration.find(marker, position)) != std::string::npos) {
        declaration.erase(position, marker.size());
    }
    return declaration;
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
        {4U, standalone::engine_property::allow_unsafe_references, 1U},
        {5U, standalone::engine_property::automatic_imports, 1U},
    };
    profile.host_stubs = {
        {0U, standalone::host_stub_kind::callable, 0U, 1U},
        {1U, standalone::host_stub_kind::callable, 0U, 1U},
        {2U, standalone::host_stub_kind::storage, 8U, 8U},
        {3U, standalone::host_stub_kind::callable, 0U, 1U},
        {4U, standalone::host_stub_kind::object, 0U, 1U},
        {5U, standalone::host_stub_kind::callable, 0U, 1U},
        {6U, standalone::host_stub_kind::callable, 0U, 1U},
        {7U, standalone::host_stub_kind::callable, 0U, 1U},
        {8U, standalone::host_stub_kind::callable, 0U, 1U},
        {9U, standalone::host_stub_kind::callable, 0U, 1U},
        {10U, standalone::host_stub_kind::callable, 0U, 1U},
        {11U, standalone::host_stub_kind::callable, 0U, 1U},
        {12U, standalone::host_stub_kind::callable, 0U, 1U},
        {13U, standalone::host_stub_kind::callable, 0U, 1U},
        {14U, standalone::host_stub_kind::callable, 0U, 1U},
        {15U, standalone::host_stub_kind::callable, 0U, 1U},
        {16U, standalone::host_stub_kind::callable, 0U, 1U},
        {17U, standalone::host_stub_kind::callable, 0U, 1U},
        {18U, standalone::host_stub_kind::callable, 0U, 1U},
        {19U, standalone::host_stub_kind::callable, 0U, 1U},
        {20U, standalone::host_stub_kind::callable, 0U, 1U},
        {21U, standalone::host_stub_kind::callable, 0U, 1U},
        {22U, standalone::host_stub_kind::callable, 0U, 1U},
        {23U, standalone::host_stub_kind::callable, 0U, 1U},
        {24U, standalone::host_stub_kind::callable, 0U, 1U},
        {25U, standalone::host_stub_kind::callable, 0U, 1U},
        {26U, standalone::host_stub_kind::callable, 0U, 1U},
        {27U, standalone::host_stub_kind::callable, 0U, 1U},
    };
    const standalone::primitive_type primitive_types[] = {
        standalone::primitive_type::bool_type,
        standalone::primitive_type::int8,
        standalone::primitive_type::int16,
        standalone::primitive_type::int32,
        standalone::primitive_type::int64,
        standalone::primitive_type::uint8,
        standalone::primitive_type::uint16,
        standalone::primitive_type::uint32,
        standalone::primitive_type::uint64,
        standalone::primitive_type::float32,
        standalone::primitive_type::float64,
    };
    const std::uint32_t primitive_sizes[] = {1U, 1U, 2U, 4U, 8U, 1U, 2U, 4U, 8U, 4U, 8U};
    for (std::uint32_t index = 0U; index < 11U; ++index) {
        profile.primitive_operations.push_back(
            {index, primitive_types[index],
             pod_operations(primitive_sizes[index], primitive_sizes[index])});
    }
    profile.dynamic_script_operations.delegate = pod_operations(16U, 8U);
    profile.dynamic_script_operations.delegate.need_construct = true;
    profile.dynamic_script_operations.delegate.need_destruct = true;
    profile.dynamic_script_operations.delegate.need_copy = true;
    profile.dynamic_script_operations.delegate.can_hash_value = false;
    profile.dynamic_script_operations.multicast_delegate =
        profile.dynamic_script_operations.delegate;

    constexpr std::uint32_t value_flags = asOBJ_VALUE | asOBJ_POD | asOBJ_APP_PRIMITIVE;
    constexpr std::uint32_t text_flags = asOBJ_VALUE | asOBJ_APP_CLASS_CDAK;
    constexpr std::uint32_t template_flags =
        asOBJ_VALUE | asOBJ_APP_CLASS_CDAK | asOBJ_TEMPLATE |
        asOBJ_TEMPLATE_SUBTYPE_COVARIANT;
    constexpr std::uint32_t actor_flags = asOBJ_REF | asOBJ_NOCOUNT | asOBJ_IMPLICIT_HANDLE;

    auto value = entry(0U, standalone::registration_kind::object_type, context("Game"));
    value.logical_id = 10U;
    value.declaration = "Value";
    value.byte_size = 8U;
    value.alignment = 8U;
    value.flags = value_flags;
    value.operations = fixed_operations(8U, 8U);
    value.operations.fixed.can_hash_value = false;
    value.operations.fixed.never_requires_gc = true;
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
    enum_type.operations = fixed_operations(1U, 1U);
    enum_type.operations.fixed.need_copy = true;
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
    text.declaration = "FString";
    text.byte_size = 16U;
    text.alignment = 8U;
    text.flags = text_flags;
    text.operations = fixed_operations(16U, 8U);
    text.operations.fixed.can_create_property = false;
    text.operations.fixed.need_construct = true;
    text.operations.fixed.need_destruct = true;
    text.operations.fixed.need_copy = true;
    text.operations.fixed.can_hash_value = false;
    profile.registrations.push_back(text);

    auto factory = entry(13U, standalone::registration_kind::string_factory, context(""));
    factory.declaration = "FString";
    factory.factory_object_stub_id = 4U;
    profile.registrations.push_back(factory);

    auto template_type = entry(14U, standalone::registration_kind::object_type, context(""));
    template_type.logical_id = 16U;
    template_type.declaration = "TSubclassOf<class T>";
    template_type.byte_size = 8U;
    template_type.alignment = 8U;
    template_type.flags = template_flags;
    template_type.operations = fixed_operations(8U, 8U);
    template_type.operations.fixed.is_object_pointer = true;
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
    actor.operations = fixed_operations(8U, 8U);
    actor.operations.fixed.need_construct = true;
    actor.operations.fixed.is_object_pointer = true;
    actor.operations.fixed.requires_property = true;
    profile.registrations.push_back(actor);

    auto second_typedef = entry(18U, standalone::registration_kind::typedef_type, context(""));
    second_typedef.logical_id = 18U;
    second_typedef.name = "Index";
    second_typedef.target_declaration = "uint";
    profile.registrations.push_back(second_typedef);

    auto hash_method = entry(19U, standalone::registration_kind::object_method, context("Game"));
    hash_method.logical_id = 35U;
    hash_method.owner_type_id = 10U;
    hash_method.declaration = "uint32 Hash() const";
    hash_method.convention = standalone::call_convention::cdecl_object_last;
    hash_method.callable_stub_id = 6U;
    profile.registrations.push_back(hash_method);

    auto compare_method = entry(20U, standalone::registration_kind::object_method, context("Game"));
    compare_method.logical_id = 36U;
    compare_method.owner_type_id = 10U;
    compare_method.declaration = "int opCmp(const Value& Other) const";
    compare_method.convention = standalone::call_convention::cdecl_object_last;
    compare_method.callable_stub_id = 7U;
    profile.registrations.push_back(compare_method);

    const auto append_container = [&](
        const std::uint32_t type_ordinal,
        const std::uint32_t type_logical_id,
        const char* declaration,
        const std::uint32_t byte_size,
        const std::uint32_t flags,
        const standalone::type_operations_kind operations_kind,
        const std::uint32_t callback_logical_id,
        const std::uint32_t callback_stub_id,
        const standalone::template_validation_adapter adapter) {
        auto container = entry(
            type_ordinal, standalone::registration_kind::object_type, context(""));
        container.logical_id = type_logical_id;
        container.declaration = declaration;
        container.byte_size = byte_size;
        container.alignment = 8U;
        container.flags = flags;
        container.operations = container_operations(operations_kind);
        profile.registrations.push_back(container);

        auto container_callback = entry(
            type_ordinal + 1U, standalone::registration_kind::object_behaviour, context(""));
        container_callback.logical_id = callback_logical_id;
        container_callback.owner_type_id = type_logical_id;
        container_callback.behaviour = standalone::object_behaviour::template_callback;
        container_callback.declaration = "bool f(int&in Type, int&out ErrorMessage)";
        container_callback.convention = standalone::call_convention::cdecl_call;
        container_callback.callable_stub_id = callback_stub_id;
        container_callback.validation_adapter = adapter;
        profile.registrations.push_back(container_callback);
    };
    append_container(
        21U, 19U, "TArray<class T>", 16U, template_flags,
        standalone::type_operations_kind::t_array, 37U, 8U,
        standalone::template_validation_adapter::t_array);
    append_container(
        23U, 22U, "TSet<class T>", 80U, template_flags,
        standalone::type_operations_kind::t_set, 38U, 9U,
        standalone::template_validation_adapter::t_set);
    append_container(
        25U, 23U, "TMap<class K, class V>", 80U, template_flags,
        standalone::type_operations_kind::t_map, 39U, 10U,
        standalone::template_validation_adapter::t_map);
    append_container(
        27U, 24U, "TOptional<class T>", 1U,
        template_flags | asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE,
        standalone::type_operations_kind::t_optional, 40U, 11U,
        standalone::template_validation_adapter::t_optional);

    auto empty = entry(29U, standalone::registration_kind::object_type, context(""));
    empty.logical_id = 25U;
    empty.declaration = "Empty";
    empty.byte_size = 1U;
    empty.alignment = 1U;
    empty.flags = value_flags;
    empty.operations = fixed_operations(0U, 1U);
    profile.registrations.push_back(empty);

    auto no_copy = entry(30U, standalone::registration_kind::object_type, context(""));
    no_copy.logical_id = 26U;
    no_copy.declaration = "NoCopy";
    no_copy.byte_size = 8U;
    no_copy.alignment = 8U;
    no_copy.flags = value_flags;
    no_copy.operations = fixed_operations(8U, 8U);
    no_copy.operations.fixed.can_copy = false;
    profile.registrations.push_back(no_copy);

    const auto append_callable = [&](const std::uint32_t ordinal,
                                     const standalone::registration_kind kind,
                                     const std::uint32_t logical_id,
                                     const std::uint32_t owner_type_id,
                                     const char* declaration,
                                     const standalone::call_convention convention,
                                     const std::uint32_t stub_id,
                                     const standalone::object_behaviour behaviour =
                                         standalone::object_behaviour::construct) {
        auto callable = entry(ordinal, kind, context(""));
        callable.logical_id = logical_id;
        callable.owner_type_id = owner_type_id;
        callable.declaration = declaration;
        callable.convention = convention;
        callable.callable_stub_id = stub_id;
        callable.behaviour = behaviour;
        profile.registrations.push_back(callable);
    };
    append_callable(31U, standalone::registration_kind::object_behaviour, 41U, 19U,
        "void f()", standalone::call_convention::cdecl_object_first, 12U);
    append_callable(32U, standalone::registration_kind::object_behaviour, 42U, 19U,
        "void f()", standalone::call_convention::cdecl_object_first, 13U,
        standalone::object_behaviour::destruct);
    append_callable(33U, standalone::registration_kind::object_method, 43U, 19U,
        "TArray<T>& opAssign(const TArray<T>& Other)",
        standalone::call_convention::cdecl_object_first, 14U);
    append_callable(34U, standalone::registration_kind::object_method, 44U, 19U,
        "void Add(const T&in if_handle_then_const Value)",
        standalone::call_convention::cdecl_object_first, 15U);

    auto fname = entry(35U, standalone::registration_kind::object_type, context(""));
    fname.logical_id = 27U;
    fname.declaration = "FName";
    fname.byte_size = 8U;
    fname.alignment = 8U;
    fname.flags = asOBJ_VALUE | asOBJ_POD | asOBJ_APP_PRIMITIVE;
    fname.operations = fixed_operations(8U, 4U);
    profile.registrations.push_back(fname);
    append_callable(36U, standalone::registration_kind::object_behaviour, 45U, 27U,
        "void f()", standalone::call_convention::cdecl_object_first, 16U);
    append_callable(37U, standalone::registration_kind::object_behaviour, 46U, 27U,
        "void f(const FName& Other)", standalone::call_convention::cdecl_object_first, 17U);
    append_callable(38U, standalone::registration_kind::object_method, 47U, 27U,
        "bool opEquals(const FName& Other) const",
        standalone::call_convention::cdecl_object_first, 18U);
    append_callable(39U, standalone::registration_kind::global_function, 48U, 0U,
        "const FName& __STATIC_NAME(int Id) no_discard",
        standalone::call_convention::cdecl_call, 19U);

    append_callable(40U, standalone::registration_kind::object_behaviour, 49U, 15U,
        "void f()", standalone::call_convention::cdecl_object_first, 20U);
    append_callable(41U, standalone::registration_kind::object_behaviour, 50U, 15U,
        "void f(const FString& Other)", standalone::call_convention::cdecl_object_first, 21U);
    append_callable(42U, standalone::registration_kind::object_behaviour, 51U, 15U,
        "void f()", standalone::call_convention::cdecl_object_first, 22U,
        standalone::object_behaviour::destruct);
    append_callable(43U, standalone::registration_kind::object_method, 52U, 15U,
        "FString& opAssign(const FString& Other)",
        standalone::call_convention::thiscall, 23U);
    append_callable(44U, standalone::registration_kind::object_method, 53U, 19U,
        "T& opIndex(int __any_implicit_integer Index)",
        standalone::call_convention::cdecl_object_first, 24U);
    append_callable(45U, standalone::registration_kind::object_method, 54U, 19U,
        "void SetNum(int32 __any_implicit_integer NewNum = 0)",
        standalone::call_convention::cdecl_object_first, 25U);
    append_callable(46U, standalone::registration_kind::object_behaviour, 55U, 16U,
        "void f(Actor Class)", standalone::call_convention::cdecl_object_first, 26U);
    append_callable(47U, standalone::registration_kind::global_function, 56U, 0U,
        "void AcceptSubclass(TSubclassOf<Actor> Class)",
        standalone::call_convention::cdecl_call, 27U);

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

    profile.final_states.push_back(type_state(15U, 16U, 8U, text_flags));
    auto template_state = type_state(16U, 8U, 8U, template_flags);
    template_state.has_implicit_constructors = true;
    template_state.accepts_value_subtype = true;
    template_state.accepts_reference_subtype = true;
    profile.final_states.push_back(template_state);
    profile.final_states.push_back(function_state(34U));
    profile.final_states.push_back(type_state(17U, 0U, 8U, actor_flags));
    profile.final_states.push_back(function_state(35U, asTRAIT_CONST));
    profile.final_states.push_back(function_state(36U, asTRAIT_CONST));
    auto array_state = type_state(19U, 16U, 8U, template_flags);
    array_state.accepts_value_subtype = true;
    array_state.accepts_reference_subtype = true;
    profile.final_states.push_back(array_state);
    profile.final_states.push_back(function_state(37U));
    auto set_state = type_state(22U, 80U, 8U, template_flags);
    set_state.accepts_value_subtype = true;
    set_state.accepts_reference_subtype = true;
    profile.final_states.push_back(set_state);
    profile.final_states.push_back(function_state(38U));
    auto map_state = type_state(23U, 80U, 8U, template_flags);
    map_state.accepts_value_subtype = true;
    map_state.accepts_reference_subtype = true;
    profile.final_states.push_back(map_state);
    profile.final_states.push_back(function_state(39U));
    auto optional_state = type_state(
        24U, 1U, 8U, template_flags | asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE);
    optional_state.accepts_value_subtype = true;
    optional_state.accepts_reference_subtype = true;
    profile.final_states.push_back(optional_state);
    profile.final_states.push_back(function_state(40U));
    profile.final_states.push_back(type_state(25U, 1U, 1U, value_flags));
    profile.final_states.push_back(type_state(26U, 8U, 8U, value_flags));
    profile.final_states.push_back(function_state(41U));
    auto array_destruct_state = function_state(42U);
    array_destruct_state.first_param = standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(array_destruct_state);
    auto array_assign_state = function_state(43U);
    array_assign_state.first_param = standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(array_assign_state);
    auto array_add_state = function_state(44U);
    array_add_state.first_param = standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(array_add_state);
    profile.final_states.push_back(type_state(
        27U, 8U, 4U, asOBJ_VALUE | asOBJ_POD | asOBJ_APP_PRIMITIVE));
    profile.final_states.push_back(function_state(45U));
    profile.final_states.push_back(function_state(46U));
    profile.final_states.push_back(function_state(47U, asTRAIT_CONST));
    profile.final_states.push_back(function_state(48U));
    profile.final_states.push_back(function_state(49U));
    profile.final_states.push_back(function_state(50U));
    profile.final_states.push_back(function_state(51U));
    profile.final_states.push_back(function_state(52U));
    auto array_index_state = function_state(53U);
    array_index_state.first_param = standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(array_index_state);
    auto array_set_num_state = function_state(54U);
    array_set_num_state.first_param = standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(array_set_num_state);
    auto subclass_constructor_state = function_state(
        55U, asTRAIT_CONSTRUCTOR | asTRAIT_PROPERTY |
            asTRAIT_IMPLICITCONSTRUCTOR | asTRAIT_GENERIC_TEMPLATE_FUNCTION);
    subclass_constructor_state.first_param =
        standalone::first_param_metadata::script_object_type;
    profile.final_states.push_back(subclass_constructor_state);
    profile.final_states.push_back(function_state(56U));
    return profile;
}

bool capture_expected(standalone::registry_profile& profile) {
    asIScriptEngine* engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return false;
    probe_string_factory factory;
    std::map<std::uint32_t, int> types;
    std::map<std::uint32_t, std::string> declarations;
    if (engine->SetEngineProperty(asEP_OPTIMIZE_BYTECODE, 1U) < 0 ||
        engine->SetEngineProperty(asEP_USE_CHARACTER_LITERALS, 1U) < 0 ||
        engine->SetEngineProperty(asEP_PROPERTY_ACCESSOR_MODE, 3U) < 0 ||
        engine->SetEngineProperty(asEP_ALLOW_IMPLICIT_HANDLE_TYPES, 1U) < 0 ||
        engine->SetEngineProperty(asEP_ALLOW_UNSAFE_REFERENCES, 1U) < 0 ||
        engine->SetEngineProperty(asEP_AUTOMATIC_IMPORTS, 1U) < 0) {
        engine->ShutDownAndRelease();
        return false;
    }
    const auto set_context = [&](const standalone::registration_entry& item) {
        if (engine->SetDefaultNamespace(item.context.name_space.c_str()) < 0) return false;
        engine->SetDefaultAccessMask(item.context.access_mask);
        return true;
    };
    const auto call_convention = [](const standalone::call_convention convention) {
        switch (convention) {
        case standalone::call_convention::cdecl_call: return asCALL_CDECL;
        case standalone::call_convention::stdcall_call: return asCALL_STDCALL;
        case standalone::call_convention::thiscall: return asCALL_THISCALL;
        case standalone::call_convention::cdecl_object_last: return asCALL_CDECL_OBJLAST;
        case standalone::call_convention::cdecl_object_first: return asCALL_CDECL_OBJFIRST;
        case standalone::call_convention::generic: return asCALL_GENERIC;
        case standalone::call_convention::thiscall_as_global: return asCALL_THISCALL_ASGLOBAL;
        case standalone::call_convention::thiscall_object_last: return asCALL_THISCALL_OBJLAST;
        case standalone::call_convention::thiscall_object_first: return asCALL_THISCALL_OBJFIRST;
        }
        return asCALL_CDECL;
    };
    const auto callable = [](const standalone::call_convention convention) {
        return convention == standalone::call_convention::thiscall
            ? asMETHOD(inert_object, method) : asFUNCTION(inert_global);
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
                declarations[item.logical_id] = callable_declaration(item.declaration);
                const int declared_id = engine->GetTypeIdByDecl(
                    declarations.at(item.logical_id).c_str());
                const int public_id = declared_id >= 0 ? declared_id : type->GetTypeId();
                types[item.logical_id] = public_id;
                append_result(profile, standalone::registration_result_kind::object_type, public_id);
            }
            break;
        }
        case standalone::registration_kind::interface_type: {
            const asUINT before = engine->GetObjectTypeCount();
            code = engine->RegisterInterface(item.declaration.c_str());
            if (code >= 0) {
                asITypeInfo* type = engine->GetObjectTypeByIndex(before);
                types[item.logical_id] = type->GetTypeId();
                declarations[item.logical_id] = item.declaration;
                append_result(profile, standalone::registration_result_kind::interface_type, type->GetTypeId());
            }
            break;
        }
        case standalone::registration_kind::interface_method:
            code = engine->RegisterInterfaceMethod(
                declarations.at(item.owner_type_id).c_str(), item.declaration.c_str());
            if (code >= 0) append_result(profile, standalone::registration_result_kind::interface_method, code, owner());
            break;
        case standalone::registration_kind::object_property: {
            asITypeInfo* type = engine->GetTypeInfoById(owner());
            const asUINT index = type->GetPropertyCount();
            code = engine->RegisterObjectProperty(
                declarations.at(item.owner_type_id).c_str(), item.declaration.c_str(),
                item.byte_offset, 0, false, item.accessor_type, item.is_protected);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::object_property, 0U, owner(), index);
            break;
        }
        case standalone::registration_kind::object_method:
            code = engine->RegisterObjectMethod(
                declarations.at(item.owner_type_id).c_str(), item.declaration.c_str(),
                callable(item.convention), call_convention(item.convention));
            if (code >= 0) append_result(profile, standalone::registration_result_kind::object_method, code, owner());
            break;
        case standalone::registration_kind::object_behaviour:
            if (item.behaviour == standalone::object_behaviour::template_callback) {
                code = engine->RegisterObjectBehaviour(
                    declarations.at(item.owner_type_id).c_str(), asBEHAVE_TEMPLATE_CALLBACK,
                    item.declaration.c_str(), asFUNCTION(class_template_validator), asCALL_CDECL);
            } else {
                code = engine->RegisterObjectBehaviour(
                    declarations.at(item.owner_type_id).c_str(),
                    item.behaviour == standalone::object_behaviour::destruct
                        ? asBEHAVE_DESTRUCT : asBEHAVE_CONSTRUCT,
                    item.declaration.c_str(), callable(item.convention),
                    call_convention(item.convention));
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
            code = engine->RegisterGlobalFunction(item.declaration.c_str(),
                callable(item.convention), call_convention(item.convention));
            if (code >= 0) append_result(profile, standalone::registration_result_kind::global_function, code);
            break;
        case standalone::registration_kind::enum_type:
            code = engine->RegisterEnum(item.declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                declarations[item.logical_id] = item.declaration;
                append_result(profile, standalone::registration_result_kind::enum_type, code);
            }
            break;
        case standalone::registration_kind::enum_value: {
            asITypeInfo* type = engine->GetTypeInfoById(owner());
            const asUINT index = type->GetEnumValueCount();
            code = engine->RegisterEnumValue(
                declarations.at(item.owner_type_id).c_str(), item.name.c_str(), item.enum_value);
            if (code >= 0) append_result(profile, standalone::registration_result_kind::enum_value, 0U, owner(), index);
            break;
        }
        case standalone::registration_kind::funcdef:
            code = engine->RegisterFuncdef(item.declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                declarations[item.logical_id] = item.declaration;
                append_result(profile, standalone::registration_result_kind::funcdef, code);
            }
            break;
        case standalone::registration_kind::typedef_type:
            code = engine->RegisterTypedef(item.name.c_str(), item.target_declaration.c_str());
            if (code >= 0) {
                types[item.logical_id] = code;
                declarations[item.logical_id] = item.name;
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

bool qualification_adapter_executes(
    const standalone::registry_profile& profile,
    const standalone::qualification_runtime_kind kind,
    const std::vector<std::string>& static_names,
    const std::vector<std::string>& static_name_identities,
    const char* const source,
    const char* const declaration) {
    asIScriptEngine* const engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    if (engine == nullptr) return false;
    standalone::registry_runtime runtime;
    std::string detail;
    if (!runtime.configure_qualification_runtime(
            kind, static_names, static_name_identities, detail)) {
        std::cerr << detail << '\n';
        engine->ShutDownAndRelease();
        return false;
    }
    const auto replay = standalone::replay_registry(*engine, profile, runtime);
    if (!replay.succeeded() || !runtime.qualification_runtime_ready(detail)) {
        std::cerr << (replay.succeeded() ? detail : replay.detail) << '\n';
        engine->ShutDownAndRelease();
        return false;
    }
    message_log diagnostics;
    engine->SetMessageCallback(
        asFUNCTION(message_log::receive), &diagnostics, asCALL_CDECL);
    asIScriptModule* const module = engine->GetModule("qualification", asGM_ALWAYS_CREATE);
    if (module == nullptr || module->AddScriptSection(
            "qualification", source, std::strlen(source)) < 0 ||
        standalone::build_module(*module) < 0) {
        std::cerr << "qualification compile failed for kind "
                  << static_cast<int>(kind) << '\n';
        for (const auto& message : diagnostics.messages) std::cerr << message << '\n';
        if (asITypeInfo* type = engine->GetTypeInfoByDecl("TArray<int32>")) {
            for (asUINT index = 0; index < type->GetMethodCount(); ++index) {
                std::cerr << "qualification method: "
                          << type->GetMethodByIndex(index)->GetDeclaration() << '\n';
            }
        }
        engine->ShutDownAndRelease();
        return false;
    }
    asIScriptFunction* const function = module->GetFunctionByDecl(declaration);
    asCScriptFunction* concrete_array_assign = nullptr;
    if (kind == standalone::qualification_runtime_kind::t_array_int32) {
        asITypeInfo* const concrete_array = engine->GetTypeInfoByDecl("TArray<int32>");
        concrete_array_assign = concrete_array == nullptr ? nullptr :
            static_cast<asCScriptFunction*>(concrete_array->GetMethodByDecl(
                "TArray<int32>& opAssign(const TArray<int32>& Other)"));
        if (concrete_array_assign == nullptr || concrete_array_assign->sysFuncIntf == nullptr) {
            engine->ShutDownAndRelease();
            return false;
        }
        // Reproduce the donor's concrete-template clone path: the exact adapter caller is
        // retained, but ScriptObjectType metadata is dropped on the generated function.
        concrete_array_assign->sysFuncIntf->passFirstParamMetaData =
            asEFirstParamMetaData::None;
    }
    if (!runtime.prepare_qualification_runtime(*engine, detail) ||
        (concrete_array_assign != nullptr &&
         concrete_array_assign->sysFuncIntf->passFirstParamMetaData !=
            asEFirstParamMetaData::ScriptObjectType)) {
        std::cerr << detail << '\n';
        engine->ShutDownAndRelease();
        return false;
    }
    if (function == nullptr ||
        runtime.qualification_function_allowed(
            engine->GetGlobalFunctionByDecl("void Log(int value)")) ||
        runtime.qualification_object_type_allowed(engine->GetTypeInfoByDecl("Game::Value")) ||
        runtime.qualification_global_address_allowed(&runtime)) {
        engine->ShutDownAndRelease();
        return false;
    }
    asUINT word_count = 0U;
    const asDWORD* const bytecode = function->GetByteCode(&word_count);
    for (std::size_t offset = 0U; bytecode != nullptr && offset < word_count;) {
        const asBYTE opcode = *reinterpret_cast<const asBYTE*>(bytecode + offset);
        if (!runtime.qualification_instruction_allowed(
                *engine, bytecode + offset, opcode, detail)) {
            std::cerr << "qualification kind " << static_cast<int>(kind)
                      << ": " << detail;
            if (opcode == asBC_CALLSYS || opcode == asBC_Thiscall1) {
                const auto* called = reinterpret_cast<const asIScriptFunction*>(
                    asBC_PTRARG(bytecode + offset));
                std::cerr << " (" << (called == nullptr ? "<null>" : called->GetDeclaration())
                          << ")";
            }
            std::cerr << '\n';
            engine->ShutDownAndRelease();
            return false;
        }
        const int words = asBCTypeSize[asBCInfo[opcode].type];
        if (words <= 0 || offset + static_cast<std::size_t>(words) > word_count) {
            engine->ShutDownAndRelease();
            return false;
        }
        offset += static_cast<std::size_t>(words);
    }
    asIScriptContext* const context = engine->CreateContext();
    if (function == nullptr || context == nullptr || context->Prepare(function) < 0 ||
        context->Execute() != asEXECUTION_FINISHED) {
        std::cerr << "qualification execute failed for kind "
                  << static_cast<int>(kind) << ": "
                  << (context == nullptr || context->GetExceptionString() == nullptr
                          ? "<no exception>" : context->GetExceptionString()) << '\n';
        if (context != nullptr) context->Release();
        engine->ShutDownAndRelease();
        return false;
    }
    bool matched = false;
    if (kind == standalone::qualification_runtime_kind::t_array_int32) {
        std::vector<std::int32_t> values;
        matched = runtime.read_qualification_tarray_int32(
            context->GetReturnObject(), values, detail) &&
            values == std::vector<std::int32_t>{0, 0};
    } else if (kind == standalone::qualification_runtime_kind::fname_equivalence) {
        matched = context->GetReturnByte() != 0U;
    } else if (kind == standalone::qualification_runtime_kind::fstring_roundtrip) {
        std::string value;
        matched = runtime.read_qualification_fstring(
            context->GetReturnObject(), value, detail) && value == "Grüße_日本";
    }
    if (!matched) {
        std::cerr << "qualification result mismatch for kind "
                  << static_cast<int>(kind) << ": " << detail << '\n';
    }
    context->Release();
    engine->ShutDownAndRelease();
    return matched;
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
    message_log diagnostics;
    if (engine->SetMessageCallback(
            asFUNCTION(message_log::receive), &diagnostics, asCALL_CDECL) < 0) {
        return fail("could not install registry smoke diagnostic callback", engine);
    }
    const auto property_mode = engine->GetEngineProperty(asEP_PROPERTY_ACCESSOR_MODE);
    const auto automatic_imports = engine->GetEngineProperty(asEP_AUTOMATIC_IMPORTS);
    const int value_type = engine->GetTypeIdByDecl("Game::Value");
    const int valid_template = engine->GetTypeIdByDecl("TSubclassOf<Actor>");
    const int invalid_template = engine->GetTypeIdByDecl("TSubclassOf<int>");
    const int default_array = engine->GetDefaultArrayTypeId();
    const int string_type = engine->GetStringFactoryReturnTypeId();
    const int valid_array = engine->GetTypeIdByDecl("TArray<Game::Value>");
    const int wrapped_array = engine->GetTypeIdByDecl("TArray<TSubclassOf<Actor>>");
    const int primitive_array = engine->GetTypeIdByDecl("TArray<int32>");
    const int valid_set = engine->GetTypeIdByDecl("TSet<Game::Value>");
    const int valid_map = engine->GetTypeIdByDecl("TMap<Game::Value, int>");
    const int valid_optional = engine->GetTypeIdByDecl("TOptional<Game::Value>");
    asITypeInfo* array_base = engine->GetTypeInfoByName("TArray");
    asITypeInfo* value_info = engine->GetTypeInfoByDecl("Game::Value");
    auto* compare = value_info == nullptr ? nullptr : static_cast<asCScriptFunction*>(
        value_info->GetMethodByDecl("int opCmp(const Value& Other) const"));
    if (property_mode != 3U || automatic_imports != 1U || value_type < 0 || valid_template < 0 ||
        invalid_template >= 0 || default_array < 0 || string_type < 0 ||
        valid_array < 0 || wrapped_array < 0 || primitive_array < 0 || valid_set < 0 || valid_map < 0 ||
        valid_optional < 0 || compare == nullptr || !compare->isInUse) {
        std::cerr << "property=" << property_mode
                  << " automatic_imports=" << automatic_imports << " value=" << value_type
                  << " valid=" << valid_template << " invalid=" << invalid_template
                  << " default_array=" << default_array << " string=" << string_type
                  << " array=" << valid_array << " wrapped_array=" << wrapped_array
                  << " set=" << valid_set
                  << " primitive_array=" << primitive_array
                  << " map=" << valid_map << " optional=" << valid_optional
                  << " base=" << array_base
                  << " base_flags=" << (array_base == nullptr ? 0U : array_base->GetFlags())
                  << '\n';
        if (value_info != nullptr) {
            for (asUINT method_index = 0U; method_index < value_info->GetMethodCount(); ++method_index) {
                asIScriptFunction* method_info = value_info->GetMethodByIndex(method_index);
                std::cerr << "method[" << method_index << "]="
                          << (method_info == nullptr ? "<null>" : method_info->GetDeclaration()) << '\n';
            }
        }
        return fail("replayed registry is not usable or template validation changed", engine);
    }

    constexpr const char* script_types =
        "struct ScriptValue { int Value; }\n"
        "class ScriptReference { int Value; }\n"
        "enum EScript { Ready }\n"
        "void UseArray(TArray<ScriptValue>& Value) {}\n"
        "void UseSet(TSet<EScript>& Value) {}\n"
        "void UseOptional(TOptional<ScriptValue>& Value) {}\n";
    asIScriptModule* script_module = engine->GetModule("script-types", asGM_ALWAYS_CREATE);
    diagnostics.messages.clear();
    if (script_module != nullptr) {
        script_module->AddPreClassData("ScriptReference", asPreClassData{});
    }
    const int add_script_types = script_module == nullptr ? asERROR :
        script_module->AddScriptSection(
            "script-types", script_types, std::strlen(script_types));
    const int build_script_types = add_script_types < 0 ? asERROR :
        standalone::build_module(*script_module);
    if (script_module == nullptr || add_script_types < 0 || build_script_types < 0) {
        for (const std::string& message : diagnostics.messages) std::cerr << message << '\n';
        std::cerr << "script build=" << build_script_types << '\n';
        return fail("dynamic script struct or enum operations did not resolve", engine);
    }

    constexpr const char* implicit_subclass_conversion =
        "void ConvertSubclass(Actor Class) { AcceptSubclass(Class); }\n";
    asIScriptModule* conversion_module =
        engine->GetModule("implicit-subclass-conversion", asGM_ALWAYS_CREATE);
    diagnostics.messages.clear();
    const int add_conversion = conversion_module == nullptr ? asERROR :
        conversion_module->AddScriptSection(
            "implicit-subclass-conversion", implicit_subclass_conversion,
            std::strlen(implicit_subclass_conversion));
    const int build_conversion = add_conversion < 0 ? asERROR :
        standalone::build_module(*conversion_module);
    if (conversion_module == nullptr || add_conversion < 0 || build_conversion < 0) {
        for (const std::string& message : diagnostics.messages) std::cerr << message << '\n';
        return fail(
            "registration-time implicit template conversion state was not preserved",
            engine);
    }

    const auto capabilities_match = [&](const char* declaration,
                                        const bool can_create_property,
                                        const bool never_requires_gc,
                                        const bool requires_property,
                                        asIScriptModule* const module = nullptr) {
        standalone::class_generator_type_capabilities capabilities;
        std::string detail;
        const int type_id = module == nullptr ? engine->GetTypeIdByDecl(declaration) :
            module->GetTypeIdByDecl(declaration);
        if (type_id <= 0 || !runtime.resolve_class_generator_type_capabilities(
                *engine, type_id, capabilities, detail)) {
            std::cerr << "class-generator capabilities unavailable for " << declaration
                      << ": " << detail << '\n';
            return false;
        }
        if (capabilities.can_create_property != can_create_property ||
            capabilities.never_requires_gc != never_requires_gc ||
            capabilities.requires_property != requires_property) {
            std::cerr << "class-generator capability mismatch for " << declaration
                      << ": can_create=" << capabilities.can_create_property
                      << " never_gc=" << capabilities.never_requires_gc
                      << " requires=" << capabilities.requires_property << '\n';
            return false;
        }
        return true;
    };
    if (!capabilities_match("int32", true, false, false) ||
        !capabilities_match("Game::Value", true, true, false) ||
        !capabilities_match("FString", false, false, false) ||
        !capabilities_match("Actor", true, false, true) ||
        !capabilities_match("ScriptValue", true, false, false, script_module) ||
        !capabilities_match("ScriptReference", true, false, false, script_module) ||
        !capabilities_match("EScript", true, false, false, script_module) ||
        !capabilities_match("TArray<Game::Value>", true, false, false) ||
        !capabilities_match("TOptional<Game::Value>", true, false, false) ||
        !capabilities_match("TSet<Game::Value>", false, false, false) ||
        !capabilities_match("TMap<Game::Value, int>", false, false, false)) {
        return fail("class-generator type capabilities diverged from captured operations", engine);
    }
    standalone::class_generator_type_capabilities invalid_capabilities{true, true, true};
    std::string invalid_capability_detail;
    if (runtime.resolve_class_generator_type_capabilities(
            *engine, -1, invalid_capabilities, invalid_capability_detail) ||
        invalid_capability_detail.empty()) {
        return fail("invalid class-generator capability request did not fail closed", engine);
    }

    const auto rejected_source = [&](
        const char* module_name,
        const char* source,
        const std::optional<std::string_view> expected_message) {
        diagnostics.messages.clear();
        asIScriptModule* module = engine->GetModule(module_name, asGM_ALWAYS_CREATE);
        if (module == nullptr ||
            module->AddScriptSection(module_name, source, std::strlen(source)) < 0) {
            return false;
        }
        const int build = standalone::build_module(*module);
        return build < 0 &&
            (!expected_message.has_value() || diagnostics.contains(*expected_message));
    };
    if (!rejected_source(
            "nested-container", "void Bad(TArray<TArray<int32>>& Value) {}",
            "Containers cannot be nested in other containers") ||
        !rejected_source(
            "empty-array", "void Bad(TArray<Empty>& Value) {}",
            "Subtype is an empty struct") ||
        !rejected_source(
            "missing-hash", "void Bad(TSet<FString>& Value) {}",
            "Subtype cannot be constructed or copied") ||
        !rejected_source(
            "missing-copy", "void Bad(TOptional<NoCopy>& Value) {}", std::nullopt)) {
        for (const std::string& message : diagnostics.messages) std::cerr << message << '\n';
        return fail("container validator error path diverged from the pinned fork", engine);
    }
    const int invalid_nested = engine->GetTypeIdByDecl("TArray<TArray<int32>>");
    const int invalid_empty = engine->GetTypeIdByDecl("TArray<Empty>");
    const int invalid_hash = engine->GetTypeIdByDecl("TSet<FString>");
    const int invalid_copy = engine->GetTypeIdByDecl("TOptional<NoCopy>");
    if (invalid_nested >= 0 || invalid_empty >= 0 || invalid_hash >= 0 || invalid_copy >= 0) {
        return fail("invalid container instance became reflectable", engine);
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
        {28U, standalone::host_stub_kind::callable, 0U, 1U});
    if (!rejected_before_mutation(unreferenced_stub)) {
        return fail("unreferenced host stub did not fail before mutation", engine);
    }

    standalone::registry_profile incomplete_snapshot = profile;
    incomplete_snapshot.final_states.pop_back();
    if (!rejected_before_mutation(incomplete_snapshot)) {
        return fail("incomplete final-state snapshot did not fail before mutation", engine);
    }

    standalone::registry_profile mismatched_adapter = profile;
    mismatched_adapter.registrations[15].validation_adapter =
        standalone::template_validation_adapter::t_array;
    if (!rejected_before_mutation(mismatched_adapter)) {
        return fail("mismatched container validator did not reject before mutation", engine);
    }

    if (!qualification_adapter_executes(profile,
            standalone::qualification_runtime_kind::t_array_int32, {}, {},
            "TArray<int32> QualificationInvokeArray() { TArray<int32> Values; "
            "Values.SetNum(2); return Values; }",
            "TArray<int32> QualificationInvokeArray()") ||
        !qualification_adapter_executes(profile,
            standalone::qualification_runtime_kind::fname_equivalence,
            {"Äquivalent"}, {"ue5-fname-comparison-index-v1:00a07436"},
            "bool QualificationFNameEquivalent() { return __STATIC_NAME(0) == __STATIC_NAME(0); }",
            "bool QualificationFNameEquivalent()") ||
        !qualification_adapter_executes(profile,
            standalone::qualification_runtime_kind::fstring_roundtrip, {}, {},
            "FString QualificationStringRoundtrip() { FString Local = \"Grüße_日本\"; return Local; }",
            "FString QualificationStringRoundtrip()")) {
        return fail("qualification-only donor ABI adapter failed", engine);
    }
    standalone::registry_profile drifted_qualification = profile;
    const auto set_num = std::find_if(drifted_qualification.registrations.begin(),
        drifted_qualification.registrations.end(), [](const auto& registration) {
            return registration.declaration ==
                "void SetNum(int32 __any_implicit_integer NewNum = 0)";
        });
    if (set_num == drifted_qualification.registrations.end()) {
        return fail("qualification SetNum fixture registration is absent", engine);
    }
    set_num->convention = standalone::call_convention::cdecl_object_last;
    asIScriptEngine* const drift_engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
    standalone::registry_runtime drift_runtime;
    std::string drift_detail;
    if (drift_engine == nullptr || !drift_runtime.configure_qualification_runtime(
            standalone::qualification_runtime_kind::t_array_int32, {}, {}, drift_detail)) {
        if (drift_engine != nullptr) drift_engine->ShutDownAndRelease();
        return fail("could not stage qualification drift fixture", engine);
    }
    const auto drift_replay = standalone::replay_registry(
        *drift_engine, drifted_qualification, drift_runtime);
    if (!drift_replay.succeeded() || drift_runtime.qualification_runtime_ready(drift_detail)) {
        drift_engine->ShutDownAndRelease();
        return fail("qualification adapter accepted a call-convention drift", engine);
    }
    drift_engine->ShutDownAndRelease();

    std::cout << "registry profile replayed every registration class, post-bind state, "
                 "string storage, all nine template validators, qualification-only donor ABI "
                 "adapters, and fail-closed preflight\n";
    engine->ShutDownAndRelease();
    return 0;
}
