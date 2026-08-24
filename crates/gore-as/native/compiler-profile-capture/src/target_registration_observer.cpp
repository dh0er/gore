#include "target_registration_observer.hpp"

#include "live_bootstrap_internal.hpp"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <span>
#include <string_view>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::uint32_t kPublicObjectFlagMask = 0x003f'ffffu;
constexpr std::uint32_t kMaximumItems = 2'000'000;
constexpr std::uint32_t kMaximumObjectBytes = 64u * 1024u * 1024u;
constexpr std::uint32_t kMaximumOffset = 256u * 1024u * 1024u;

namespace engine_slot {
constexpr std::size_t global_property_by_index = 16;
constexpr std::size_t object_type_count = 23;
constexpr std::size_t object_type_by_index = 24;
constexpr std::size_t function_by_id = 49;
constexpr std::size_t type_id_by_declaration = 50;
constexpr std::size_t type_by_id = 53;
constexpr std::size_t type_by_declaration = 55;
}  // namespace engine_slot

namespace type_info_slot {
constexpr std::size_t type_id = 13;
constexpr std::size_t enum_value_count = 36;
constexpr std::size_t enum_value_by_index = 37;
}  // namespace type_info_slot

constexpr std::array<std::string_view, 9> kCallConventions{
    "cdecl",          "stdcall",              "thiscall_as_global",
    "thiscall",       "cdecl_object_last",    "cdecl_object_first",
    "generic",        "thiscall_object_last", "thiscall_object_first"};
constexpr std::array<std::string_view, 14> kBehaviours{
    "construct",       "list_construct", "destruct",       "factory",
    "list_factory",    "add_ref",        "release",        "get_weakref_flag",
    "template_callback", "get_ref_count",   "set_gc_flag",    "get_gc_flag",
    "enum_refs",       "release_refs"};

bool readable_range(const std::uintptr_t first, const std::size_t bytes) noexcept {
  if (first == 0 || bytes == 0 || first > std::numeric_limits<std::uintptr_t>::max() - bytes) {
    return false;
  }
  auto cursor = first;
  const auto end = first + bytes;
  while (cursor < end) {
    MEMORY_BASIC_INFORMATION region{};
    if (VirtualQuery(reinterpret_cast<const void*>(cursor), &region, sizeof(region)) !=
            sizeof(region) ||
        region.State != MEM_COMMIT || (region.Protect & PAGE_GUARD) != 0) {
      return false;
    }
    const DWORD protection = region.Protect & 0xffu;
    if (protection != PAGE_READONLY && protection != PAGE_READWRITE &&
        protection != PAGE_WRITECOPY && protection != PAGE_EXECUTE_READ &&
        protection != PAGE_EXECUTE_READWRITE && protection != PAGE_EXECUTE_WRITECOPY) {
      return false;
    }
    const auto base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
    if (base > std::numeric_limits<std::uintptr_t>::max() - region.RegionSize) return false;
    const auto next = base + region.RegionSize;
    if (next <= cursor) return false;
    cursor = std::min(next, end);
  }
  return true;
}

bool image_address(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t address) noexcept {
  return image != 0 && address >= image && address - image < image_bytes;
}

bool cstring_equals(
    const char* const actual,
    const std::string_view expected) noexcept {
  const auto address = reinterpret_cast<std::uintptr_t>(actual);
  if (actual == nullptr || expected.size() == std::numeric_limits<std::size_t>::max() ||
      !readable_range(address, expected.size() + 1)) {
    return false;
  }
  return std::memcmp(actual, expected.data(), expected.size()) == 0 &&
         actual[expected.size()] == '\0';
}

template <typename Return, typename... Arguments>
RegistrationObserverError invoke_slot(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t object,
    const std::size_t slot,
    Return& result,
    Arguments... arguments) noexcept {
  if (!readable_range(object, sizeof(std::uintptr_t))) {
    return RegistrationObserverError::unreadable_target;
  }
  std::uintptr_t vtable = 0;
  std::memcpy(&vtable, reinterpret_cast<const void*>(object), sizeof(vtable));
  if (!image_address(image, image_bytes, vtable) ||
      !readable_range(vtable + slot * sizeof(std::uintptr_t), sizeof(std::uintptr_t))) {
    return RegistrationObserverError::abi_target_outside_image;
  }
  std::uintptr_t target = 0;
  std::memcpy(
      &target,
      reinterpret_cast<const void*>(vtable + slot * sizeof(std::uintptr_t)),
      sizeof(target));
  if (!image_address(image, image_bytes, target)) {
    return RegistrationObserverError::abi_target_outside_image;
  }
  using Function = Return(__fastcall*)(std::uintptr_t, Arguments...);
  Function function = nullptr;
  static_assert(sizeof(function) == sizeof(target));
  std::memcpy(&function, &target, sizeof(function));
  try {
    result = function(object, arguments...);
    return RegistrationObserverError::ok;
  } catch (...) {
    return RegistrationObserverError::unreadable_target;
  }
}

template <typename Value>
bool read_value(
    const std::uintptr_t object,
    const std::size_t offset,
    Value& value) noexcept {
  if (object > std::numeric_limits<std::uintptr_t>::max() - offset ||
      !readable_range(object + offset, sizeof(value))) {
    return false;
  }
  std::memcpy(&value, reinterpret_cast<const void*>(object + offset), sizeof(value));
  return true;
}

bool raw_contract(const RawRegistrationEntry& raw) noexcept {
  const auto found = std::find_if(
      registration::kPinnedRegistrationHooks.begin(),
      registration::kPinnedRegistrationHooks.end(),
      [&](const auto& hook) { return hook.kind == raw.kind; });
  if (found == registration::kPinnedRegistrationHooks.end() ||
      raw.engine_capability == 0 || raw.argument_count != found->argument_count) {
    return false;
  }
  for (std::size_t index = 0; index < raw.argument_count; ++index) {
    const auto& argument = raw.arguments[index];
    if (argument.semantic != found->arguments[index].semantic) return false;
    switch (argument.semantic) {
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1:
        if (argument.text_bytes == 0 || argument.text_bytes > 1024 ||
            argument.text[argument.text_bytes] != '\0' ||
            std::memchr(argument.text.data(), '\0', argument.text_bytes) != nullptr) {
          return false;
        }
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1: {
        if (argument.opaque_descriptor_bytes !=
            layout_v23300::donor::function_pointer_descriptor_bytes) {
          return false;
        }
        const auto flag = std::to_integer<std::uint8_t>(argument.opaque_descriptor[
            layout_v23300::donor::function_pointer_descriptor_flag]);
        if (flag == 0 || flag > 3) return false;
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1: {
        if (argument.opaque_descriptor_bytes !=
                layout_v23300::donor::function_caller_descriptor_bytes ||
            argument.scalar > 2) {
          return false;
        }
        std::uintptr_t descriptor_pointer = 0;
        std::memcpy(
            &descriptor_pointer, argument.opaque_descriptor.data(),
            sizeof(descriptor_pointer));
        if (descriptor_pointer != argument.pointer_capability ||
            ((argument.scalar == 0) != (descriptor_pointer == 0))) {
          return false;
        }
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1:
        if (argument.scalar >= kCallConventions.size()) return false;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1:
        if (argument.scalar > 1) return false;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1:
        if (argument.scalar >= kBehaviours.size()) return false;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1:
        if (argument.scalar > std::numeric_limits<std::uint32_t>::max()) return false;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1:
        break;
      default:
        return false;
    }
  }
  return true;
}

std::string text(const RawRegistrationArgument& argument) {
  return std::string(argument.text.data(), argument.text_bytes);
}

std::int32_t signed_scalar(const RawRegistrationArgument& argument) noexcept {
  const auto bits = static_cast<std::uint32_t>(argument.scalar);
  std::int32_t value = 0;
  std::memcpy(&value, &bits, sizeof(value));
  return value;
}

bool primitive_storage_layout(
    const std::int32_t engine_type_id,
    std::uint32_t& byte_size,
    std::uint32_t& alignment) noexcept {
  constexpr std::array<std::uint32_t, 12> sizes{
      0, 1, 1, 2, 4, 8, 1, 2, 4, 8, 4, 8};
  if (engine_type_id <= 0 ||
      static_cast<std::size_t>(engine_type_id) >= sizes.size()) {
    return false;
  }
  byte_size = sizes[static_cast<std::size_t>(engine_type_id)];
  alignment = byte_size;
  return true;
}

bool callable_capability(
    const RawRegistrationArgument& function_pointer,
    const RawRegistrationArgument& caller,
    std::uintptr_t& capability) noexcept {
  if (function_pointer.opaque_descriptor_bytes !=
          layout_v23300::donor::function_pointer_descriptor_bytes ||
      caller.opaque_descriptor_bytes !=
          layout_v23300::donor::function_caller_descriptor_bytes ||
      caller.scalar > 2) {
    return false;
  }
  std::uintptr_t function = 0;
  std::memcpy(&function, function_pointer.opaque_descriptor.data(), sizeof(function));
  capability = caller.scalar == 0 ? function : caller.pointer_capability;
  return capability != 0;
}

const char* template_adapter(const std::string_view declaration) noexcept {
  constexpr std::array<std::pair<std::string_view, const char*>, 9> adapters{{
      {"TArray<", "t_array"},
      {"TMap<", "t_map"},
      {"TSet<", "t_set"},
      {"TOptional<", "t_optional"},
      {"TSubclassOf<", "t_subclass_of"},
      {"TObjectPtr<", "t_object_ptr"},
      {"TWeakObjectPtr<", "t_weak_object_ptr"},
      {"TSoftObjectPtr<", "t_soft_object_ptr"},
      {"TSoftClassPtr<", "t_soft_class_ptr"},
  }};
  for (const auto& [prefix, adapter] : adapters) {
    if (declaration.starts_with(prefix)) return adapter;
  }
  return nullptr;
}

bool container_adapter_matches(
    const TypeOperationsJsonKind operations,
    const std::string_view adapter) noexcept {
  return (operations == TypeOperationsJsonKind::t_array && adapter == "t_array") ||
         (operations == TypeOperationsJsonKind::t_map && adapter == "t_map") ||
         (operations == TypeOperationsJsonKind::t_set && adapter == "t_set") ||
         (operations == TypeOperationsJsonKind::t_optional && adapter == "t_optional") ||
         ((operations == TypeOperationsJsonKind::fixed ||
           operations == TypeOperationsJsonKind::unavailable) &&
          adapter != "t_array" && adapter != "t_map" && adapter != "t_set" &&
          adapter != "t_optional");
}

bool add_count(std::uint32_t& value, const std::uint32_t increment = 1) noexcept {
  if (increment > kMaximumItems || value > kMaximumItems - increment) return false;
  value += increment;
  return true;
}

RegistrationObserverError serialization_error(const CaptureSerializationError error) noexcept {
  switch (error) {
    case CaptureSerializationError::ok:
      return RegistrationObserverError::ok;
    case CaptureSerializationError::unresolved_identity:
    case CaptureSerializationError::duplicate_identity:
      return RegistrationObserverError::unresolved_identity;
    case CaptureSerializationError::limit_exceeded:
      return RegistrationObserverError::limit_exceeded;
    default:
      return RegistrationObserverError::serialization_rejected;
  }
}

bool resolve_trace_type(
    void* const context,
    const std::uintptr_t capability,
    std::uint32_t& type_id) noexcept {
  auto* correlation = static_cast<const TraceIdCorrelation*>(context);
  std::uint32_t engine_id = 0;
  return correlation != nullptr &&
         correlation->type_ids(capability, type_id, engine_id) ==
             CaptureSerializationError::ok;
}

}  // namespace

TargetRegistrationObserver::TargetRegistrationObserver(
    const std::uintptr_t primary_image,
    const std::uint32_t primary_image_bytes,
    const std::uintptr_t engine_capability,
    const PointerTokenResolver pointer_tokens
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
    ,
    const RegistrationObserverTestTarget test_target
#endif
    ) noexcept
    : primary_image_(primary_image),
      primary_image_bytes_(primary_image_bytes),
      engine_capability_(engine_capability),
      pointer_tokens_(pointer_tokens)
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
      ,
      test_target_(test_target)
#endif
{}

RegistrationObserverError TargetRegistrationObserver::begin_observation(
    const RegistryCounts& baseline) noexcept {
  if (begun_ || finalized_ || primary_image_ == 0 || primary_image_bytes_ == 0 ||
      engine_capability_ == 0 || pointer_tokens_.resolve == nullptr ||
      pointer_tokens_.resolve_object == nullptr ||
      pointer_tokens_.resolve_storage == nullptr ||
      baseline.total_registrations > kMaximumItems) {
    return RegistrationObserverError::invalid_state;
  }
  baseline_ = baseline;
  begun_ = true;
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::prepare(
    const RawRegistrationEntry& raw,
    PendingRegistrationProjection& pending) const noexcept {
  if (!begun_ || finalized_ || raw.engine_capability != engine_capability_ ||
      !raw_contract(raw)) {
    return RegistrationObserverError::invalid_raw_frame;
  }
  PendingRegistrationProjection value{};
  value.raw = raw;
  if (extract_registration_context_v23300(raw.engine_capability, value.context) !=
      FinalStateError::ok) {
    return RegistrationObserverError::unreadable_target;
  }
  pending = std::move(value);
  return RegistrationObserverError::ok;
}

bool TargetRegistrationObserver::is_core_intrinsic(
    const PendingRegistrationProjection& pending,
    const std::int32_t eax_result) const noexcept {
  // AngelScript's own RegisterScriptFunction bootstrap deliberately calls the
  // public API once, then immediately renames the function and rewrites its
  // return type to the private $func handle.  The standalone donor performs
  // this same bootstrap before application registry replay, so recording this
  // call would duplicate the core intrinsic and leave an impossible public
  // final-state projection.  Pin every stable discriminator from v2.33.0 and
  // accept it only as the very first mutation of an empty engine.
  constexpr std::string_view declaration =
      "void f(int &in Ptr, int &in Ptr2)";
  const auto& raw = pending.raw;
  return begun_ && !finalized_ && baseline_.total_registrations == 0 &&
         expected_delta_.total_registrations == 0 && completed_records_.empty() &&
         types_.empty() && identities_.empty() && eax_result == 10 &&
         raw.kind == GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1 &&
         raw.argument_count == 5 && raw.arguments[0].text_bytes == declaration.size() &&
         std::string_view(raw.arguments[0].text.data(), raw.arguments[0].text_bytes) ==
             declaration &&
         raw.arguments[2].scalar == 6 && pending.context.name_space.empty() &&
         !pending.context.has_config_group;
}

RegistrationObserverError TargetRegistrationObserver::pointer_token(
    const std::uintptr_t capability,
    std::uint32_t& token) const noexcept {
  if (capability == 0 || pointer_tokens_.resolve == nullptr ||
      !pointer_tokens_.resolve(pointer_tokens_.context, capability, token) ||
      token == std::numeric_limits<std::uint32_t>::max()) {
    return RegistrationObserverError::pointer_token_rejected;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::object_pointer_token(
    const std::uintptr_t capability,
    std::uint32_t& token) const noexcept {
  if (capability == 0 || pointer_tokens_.resolve_object == nullptr ||
      !pointer_tokens_.resolve_object(pointer_tokens_.context, capability, token) ||
      token == std::numeric_limits<std::uint32_t>::max()) {
    return RegistrationObserverError::pointer_token_rejected;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::storage_pointer_token(
    const std::uintptr_t capability,
    std::uint32_t& token) const noexcept {
  if (capability == 0 || pointer_tokens_.resolve_storage == nullptr ||
      !pointer_tokens_.resolve_storage(pointer_tokens_.context, capability, token) ||
      token == std::numeric_limits<std::uint32_t>::max()) {
    return RegistrationObserverError::pointer_token_rejected;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::type_by_id(
    const std::int32_t engine_type_id,
    std::uintptr_t& capability) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.type_by_id != nullptr) {
    return test_target_.type_by_id(test_target_.context, engine_type_id, capability) &&
                   capability != 0
               ? RegistrationObserverError::ok
               : RegistrationObserverError::unresolved_identity;
  }
#endif
  const auto status = invoke_slot(
      primary_image_, primary_image_bytes_, engine_capability_,
      engine_slot::type_by_id, capability, engine_type_id);
  return status == RegistrationObserverError::ok && capability == 0
             ? RegistrationObserverError::unresolved_identity
             : status;
}

RegistrationObserverError TargetRegistrationObserver::function_by_id(
    const std::int32_t engine_function_id,
    std::uintptr_t& capability) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.function_by_id != nullptr) {
    return test_target_.function_by_id(
               test_target_.context, engine_function_id, capability) &&
                   capability != 0
               ? RegistrationObserverError::ok
               : RegistrationObserverError::unresolved_identity;
  }
#endif
  const auto status = invoke_slot(
      primary_image_, primary_image_bytes_, engine_capability_,
      engine_slot::function_by_id, capability, engine_function_id);
  return status == RegistrationObserverError::ok && capability == 0
             ? RegistrationObserverError::unresolved_identity
             : status;
}

RegistrationObserverError TargetRegistrationObserver::owner_by_declaration(
    const std::string& declaration,
    const std::string& name_space,
    std::uintptr_t& capability,
    std::uint32_t& trace_id,
    std::uint32_t& engine_id,
    TypeOperationsJsonKind& operations) const noexcept {
  const auto observed = std::find_if(
      types_.begin(), types_.end(), [&](const TypeRecord& value) {
        return value.declaration == declaration && value.name_space == name_space;
      });
  if (observed != types_.end()) {
    capability = observed->capability;
    trace_id = observed->trace_id;
    engine_id = observed->engine_id;
    operations = observed->operations;
    return RegistrationObserverError::ok;
  }
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.type_by_declaration != nullptr) {
    if (!test_target_.type_by_declaration(
            test_target_.context, declaration.c_str(), capability) || capability == 0) {
      return RegistrationObserverError::unresolved_identity;
    }
  } else
#endif
  {
    const auto status = invoke_slot(
        primary_image_, primary_image_bytes_, engine_capability_,
        engine_slot::type_by_declaration, capability, declaration.c_str());
    if (status != RegistrationObserverError::ok) return status;
    if (capability == 0) return RegistrationObserverError::owner_declaration_missing;
  }
  const auto correlation_status = correlation_.type_ids(capability, trace_id, engine_id);
  if (correlation_status != CaptureSerializationError::ok) {
    return RegistrationObserverError::owner_correlation_missing;
  }
  const auto type = std::find_if(types_.begin(), types_.end(), [&](const TypeRecord& value) {
    return value.capability == capability && value.trace_id == trace_id &&
           value.engine_id == engine_id;
  });
  if (type == types_.end()) return RegistrationObserverError::owner_record_missing;
  operations = type->operations;
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::global_property(
    const std::uint32_t index,
    std::int32_t& type_id,
    std::uintptr_t& storage,
    std::uintptr_t& property) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.global_property != nullptr) {
    return test_target_.global_property(
               test_target_.context, index, type_id, storage, property) &&
                   storage != 0 && property != 0
               ? RegistrationObserverError::ok
               : RegistrationObserverError::unresolved_identity;
  }
#endif
  std::int32_t result = -1;
  void* storage_pointer = nullptr;
  const auto status = invoke_slot(
      primary_image_, primary_image_bytes_, engine_capability_,
      engine_slot::global_property_by_index, result, index,
      static_cast<const char**>(nullptr), static_cast<const char**>(nullptr), &type_id,
      static_cast<bool*>(nullptr), static_cast<const char**>(nullptr),
      &storage_pointer, static_cast<std::uint32_t*>(nullptr));
  if (status != RegistrationObserverError::ok) return status;
  storage = reinterpret_cast<std::uintptr_t>(storage_pointer);
  if (result < 0 || storage == 0) return RegistrationObserverError::unresolved_identity;

  std::uintptr_t array = 0;
  std::uint32_t length = 0;
  std::uint32_t capacity = 0;
  if (!read_value(
          engine_capability_,
          layout_v23300::target_confirmed::target_engine_registered_global_properties,
          array) ||
      !read_value(
          engine_capability_,
          layout_v23300::target_confirmed::target_engine_registered_global_properties + 8,
          length) ||
      !read_value(
          engine_capability_,
          layout_v23300::target_confirmed::target_engine_registered_global_properties + 12,
          capacity) ||
      length > capacity || index >= length || array == 0 ||
      !read_value(array, static_cast<std::size_t>(index) * sizeof(property), property) ||
      property == 0) {
    return RegistrationObserverError::unreadable_target;
  }
  std::uintptr_t private_storage = 0;
  if (!read_value(
          property,
          layout_v23300::target_confirmed::global_property_real_address,
          private_storage) ||
      private_storage != storage) {
    return RegistrationObserverError::unresolved_identity;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::latest_global_property(
    std::uint32_t& index,
    std::int32_t& type_id,
    std::uintptr_t& storage,
    std::uintptr_t& property) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.global_property != nullptr) {
    index = 0;
    return global_property(index, type_id, storage, property);
  }
#endif
  std::uint32_t length = 0;
  if (!read_value(
          engine_capability_,
          layout_v23300::target_confirmed::target_engine_registered_global_properties + 8,
          length) ||
      length == 0) {
    return RegistrationObserverError::unreadable_target;
  }
  index = length - 1;
  return global_property(index, type_id, storage, property);
}

RegistrationObserverError TargetRegistrationObserver::object_property(
    const std::uintptr_t owner,
    const std::uint32_t index,
    std::uintptr_t& property) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.object_property != nullptr) {
    return test_target_.object_property(test_target_.context, owner, index, property) &&
                   property != 0
               ? RegistrationObserverError::ok
               : RegistrationObserverError::unresolved_identity;
  }
#endif
  std::uintptr_t array = 0;
  std::uint32_t length = 0;
  std::uint32_t capacity = 0;
  constexpr auto offset = layout_v23300::target_confirmed::object_type_properties;
  if (!read_value(owner, offset, array) || !read_value(owner, offset + 8, length) ||
      !read_value(owner, offset + 12, capacity) || length > capacity || index >= length ||
      array == 0 ||
      !read_value(array, static_cast<std::size_t>(index) * sizeof(property), property) ||
      property == 0) {
    return RegistrationObserverError::unreadable_target;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::latest_object_property(
    const std::uintptr_t owner,
    std::uint32_t& index,
    std::uintptr_t& property) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.object_property != nullptr) {
    index = 0;
    return object_property(owner, index, property);
  }
#endif
  std::uint32_t length = 0;
  constexpr auto offset = layout_v23300::target_confirmed::object_type_properties;
  if (!read_value(owner, offset + 8, length) || length == 0) {
    return RegistrationObserverError::unreadable_target;
  }
  index = length - 1;
  return object_property(owner, index, property);
}

RegistrationObserverError TargetRegistrationObserver::resolve_type_projection(
    const std::int32_t engine_type_id,
    const std::string& declaration,
    TargetTypeOperationsProjection& projection,
    bool& available) const noexcept {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (test_target_.type_operations != nullptr) {
    if (!test_target_.type_operations(
            test_target_.context, engine_type_id, declaration.c_str(), projection)) {
      return RegistrationObserverError::type_operations_rejected;
    }
    available = projection.kind != TypeOperationsJsonKind::unavailable;
    return RegistrationObserverError::ok;
  }
#endif
  const auto status = resolve_target_type_operations_projection_v1(
      primary_image_, primary_image_bytes_, engine_type_id,
      declaration.data(), declaration.size(), projection);
  if (status == TargetTypeUsageError::unresolved_type) {
    projection = {};
    // Generic container definitions intentionally return asSUCCESS rather than
    // a concrete instantiated type.  Before their first instantiation the
    // target TypeUsage resolver therefore has no value tree to project.  The
    // replay contract for the four closed built-in containers is identified
    // by the exact declaration and the separately observed template callback;
    // all other unresolved types remain unavailable.
    const auto classified = classify_target_type_operations_v1(
        declaration.data(), declaration.size());
    if (classified == TypeOperationsJsonKind::t_array ||
        classified == TypeOperationsJsonKind::t_map ||
        classified == TypeOperationsJsonKind::t_set ||
        classified == TypeOperationsJsonKind::t_optional) {
      projection.kind = classified;
      available = true;
    } else {
      available = false;
    }
    return RegistrationObserverError::ok;
  }
  if (status != TargetTypeUsageError::ok) {
    return RegistrationObserverError::type_operations_rejected;
  }
  available = true;
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::resolve_type_projection_cached(
    const std::int32_t engine_type_id,
    const std::string& declaration,
    TargetTypeOperationsProjection& projection,
    bool& available) noexcept {
  const auto found = std::find_if(
      type_projection_cache_.begin(), type_projection_cache_.end(),
      [&](const TypeProjectionRecord& value) {
        return value.engine_type_id == engine_type_id;
      });
  if (found != type_projection_cache_.end()) {
    projection = found->projection;
    available = true;
    return RegistrationObserverError::ok;
  }
  auto status = resolve_type_projection(
      engine_type_id, declaration, projection, available);
  if (status != RegistrationObserverError::ok || !available) return status;
  try {
    type_projection_cache_.push_back({engine_type_id, projection});
  } catch (...) {
    return RegistrationObserverError::limit_exceeded;
  }
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::complete(
    const std::uint32_t bind_callback_ordinal,
    const PendingRegistrationProjection& pending,
    const std::int32_t eax_result,
    CompletedRegistrationProjection& completed) noexcept {
  if (!begun_ || finalized_) return RegistrationObserverError::invalid_state;
  try {
    CompletedRegistrationProjection value{};
    // The coordinator aborts the entire capture after any observer failure;
    // there is no valid continuation that could consume a rolled-back
    // observer.  Mutating in place avoids copying every accumulated identity,
    // stub witness and final-state expectation for every registration.
    const auto status = complete_in_place(
        bind_callback_ordinal, pending, eax_result, value);
    if (status != RegistrationObserverError::ok) return status;
    completed = std::move(value);
    return RegistrationObserverError::ok;
  } catch (...) {
    return RegistrationObserverError::limit_exceeded;
  }
}

RegistrationObserverError TargetRegistrationObserver::complete_in_place(
    const std::uint32_t bind_callback_ordinal,
    const PendingRegistrationProjection& pending,
    const std::int32_t eax_result,
    CompletedRegistrationProjection& completed) noexcept {
  if (pending.raw.engine_capability != engine_capability_ || !raw_contract(pending.raw)) {
    return RegistrationObserverError::invalid_raw_frame;
  }
  RegistrationEntryJsonProjection entry{};
  RegistrationResultJsonProjection result{};
  entry.context = pending.context;
  auto status = serialization_error(
      correlation_.claim_registration(entry.ordinal, entry.registration_id));
  if (status != RegistrationObserverError::ok) return status;
  if (project_registration_post_result_v23300(
          pending.raw.kind, eax_result, result.post_result) != FinalStateError::ok) {
    return RegistrationObserverError::result_rejected;
  }
  const auto& arguments = pending.raw.arguments;
  RegistrationStubCapabilities stubs{};
  std::uintptr_t identity_capability = 0;
  std::uintptr_t owner_capability = 0;
  std::uint32_t owner_engine_id = 0;
  TypeOperationsJsonKind owner_operations = TypeOperationsJsonKind::unavailable;
  std::int32_t global_storage_type_id = -1;

  const auto resolve_owner = [&](const std::size_t argument) {
    return owner_by_declaration(
        text(arguments[argument]), entry.context.name_space,
        owner_capability, entry.owner_trace_type_id,
        owner_engine_id, owner_operations);
  };
  const auto resolve_function = [&]() {
    status = function_by_id(eax_result, identity_capability);
    if (status != RegistrationObserverError::ok) return status;
    status = serialization_error(
        correlation_.register_function(
            static_cast<std::uint32_t>(eax_result), entry.trace_id));
    if (status == RegistrationObserverError::ok) {
      identities_.push_back({IdentityKind::function, identity_capability, owner_capability,
                             0, entry.trace_id, static_cast<std::uint32_t>(eax_result),
                             0, 0, 0});
    }
    return status;
  };
  const auto callable_stubs = [&](const std::size_t function_index,
                                  const std::size_t convention_index,
                                  const std::size_t caller_index,
                                  const std::size_t auxiliary_index) {
    const auto convention = static_cast<std::uint32_t>(arguments[convention_index].scalar);
    if (convention >= kCallConventions.size()) {
      return RegistrationObserverError::invalid_raw_frame;
    }
    entry.call_convention = kCallConventions[convention];
    std::uintptr_t callable = 0;
    if (!callable_capability(arguments[function_index], arguments[caller_index], callable)) {
      return RegistrationObserverError::invalid_raw_frame;
    }
    stubs.has_callable = true;
    auto local_status = pointer_token(callable, stubs.callable_pointer_token);
    if (local_status != RegistrationObserverError::ok) return local_status;
    const auto auxiliary = arguments[auxiliary_index].pointer_capability;
    stubs.has_auxiliary_object = auxiliary != 0;
    if (stubs.has_auxiliary_object) {
      local_status = object_pointer_token(auxiliary, stubs.auxiliary_object_pointer_token);
      if (local_status != RegistrationObserverError::ok) return local_status;
    }
    return (convention == 2) == stubs.has_auxiliary_object
               ? RegistrationObserverError::ok
               : RegistrationObserverError::invalid_raw_frame;
  };

  try {
    switch (pending.raw.kind) {
      case GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1:
        entry.kind = RegistrationEntryJsonKind::global_function;
        entry.declaration = text(arguments[0]);
        status = callable_stubs(1, 2, 3, 4);
        if (status == RegistrationObserverError::ok) status = resolve_function();
        if (status == RegistrationObserverError::ok && !add_count(expected_delta_.functions)) {
          status = RegistrationObserverError::limit_exceeded;
        }
        break;
      case GORE_AS_CAPTURE_REGISTRATION_GLOBAL_PROPERTY_V1: {
        entry.kind = RegistrationEntryJsonKind::global_property;
        entry.declaration = text(arguments[0]);
        std::int32_t type_id = 0;
        std::uintptr_t storage = 0;
        std::uint32_t property_index = 0;
        status = latest_global_property(
            property_index, type_id, storage, identity_capability);
        global_storage_type_id = type_id;
        if (status != RegistrationObserverError::ok ||
            storage != arguments[1].pointer_capability) {
          return status == RegistrationObserverError::ok
                     ? RegistrationObserverError::unresolved_identity
                     : status;
        }
        TargetTypeOperationsProjection operations{};
        bool available = false;
        std::uint32_t storage_bytes = 0;
        std::uint32_t storage_alignment = 0;
        if (primitive_storage_layout(type_id, storage_bytes, storage_alignment)) {
          operations.kind = TypeOperationsJsonKind::fixed;
          operations.fixed.value_size = storage_bytes;
          operations.fixed.value_alignment = storage_alignment;
          available = true;
        } else {
          const auto registered = std::find_if(
              types_.begin(), types_.end(), [&](const auto& value) {
                return value.engine_id == static_cast<std::uint32_t>(type_id);
              });
          if (registered != types_.end() && registered->public_byte_size != 0 &&
              registered->public_alignment != 0) {
            // RegisterObjectType supplies the safe live storage extent.  The
            // complete TypeUsage table upgrades it at finalize_registry().
            operations.kind = TypeOperationsJsonKind::fixed;
            operations.fixed.value_size = registered->public_byte_size;
            operations.fixed.value_alignment = registered->public_alignment;
            available = true;
          } else {
            // Closed template instantiations are created internally and have
            // no public RegisterObjectType event. Resolve only this uncommon
            // case now, and cache it for later globals with the same type ID.
            status = resolve_type_projection_cached(
                type_id, entry.declaration, operations, available);
            if (status != RegistrationObserverError::ok) {
              return RegistrationObserverError::global_property_type_operations_failed;
            }
          }
        }
        live_capture_note_reflected_type_v1(
            type_id, static_cast<std::uint32_t>(operations.kind),
            operations.fixed.value_size, operations.fixed.value_alignment, available);
        if (!available) {
          return RegistrationObserverError::global_property_type_operations_unavailable;
        }
        if (operations.fixed.value_size == 0) {
          return RegistrationObserverError::global_property_zero_value_size;
        }
        stubs.has_storage = true;
        stubs.storage_byte_len = operations.fixed.value_size;
        stubs.storage_alignment = operations.fixed.value_alignment;
        status = storage_pointer_token(storage, stubs.storage_pointer_token);
        if (status != RegistrationObserverError::ok) return status;
        status = serialization_error(correlation_.register_global_property(
            property_index, entry.trace_id));
        if (status == RegistrationObserverError::ok) {
          result.post_result.value = property_index;
          identities_.push_back({IdentityKind::global_property, identity_capability, 0,
                                 storage, entry.trace_id, 0,
                                 property_index, 0, 0});
          if (!add_count(expected_delta_.global_properties)) {
            status = RegistrationObserverError::limit_exceeded;
          }
        }
        break;
      }
      case GORE_AS_CAPTURE_REGISTRATION_OBJECT_TYPE_V1:
      case GORE_AS_CAPTURE_REGISTRATION_INTERFACE_V1:
      case GORE_AS_CAPTURE_REGISTRATION_ENUM_V1:
      case GORE_AS_CAPTURE_REGISTRATION_FUNCDEF_V1:
      case GORE_AS_CAPTURE_REGISTRATION_TYPEDEF_V1: {
        live_capture_note_observer_stage_v1(0x300u);
        const bool object = pending.raw.kind == GORE_AS_CAPTURE_REGISTRATION_OBJECT_TYPE_V1;
        const bool interface_type = pending.raw.kind == GORE_AS_CAPTURE_REGISTRATION_INTERFACE_V1;
        const bool enumeration = pending.raw.kind == GORE_AS_CAPTURE_REGISTRATION_ENUM_V1;
        const bool funcdef = pending.raw.kind == GORE_AS_CAPTURE_REGISTRATION_FUNCDEF_V1;
        entry.kind = object ? RegistrationEntryJsonKind::object_type
                            : interface_type ? RegistrationEntryJsonKind::interface
                            : enumeration ? RegistrationEntryJsonKind::enumeration
                            : funcdef ? RegistrationEntryJsonKind::funcdef
                                      : RegistrationEntryJsonKind::type_alias;
        if (entry.kind == RegistrationEntryJsonKind::type_alias) {
          entry.name = text(arguments[0]);
          entry.target_declaration = text(arguments[1]);
        } else {
          entry.declaration = text(arguments[0]);
        }
        std::int32_t engine_type_id = eax_result;
        live_capture_note_observer_stage_v1(0x301u);
#if !defined(GORE_AS_CAPTURE_TEST_TARGET)
        constexpr std::uint32_t kObjectTemplateFlag = 1u << 6;
        const bool template_definition =
            object && eax_result == 0 &&
            (static_cast<std::uint32_t>(arguments[2].scalar) & kObjectTemplateFlag) != 0;
        if (template_definition) {
          std::uint32_t object_type_count = 0;
          status = invoke_slot(
              primary_image_, primary_image_bytes_, engine_capability_,
              engine_slot::object_type_count, object_type_count);
          if (status == RegistrationObserverError::ok && object_type_count != 0) {
            status = invoke_slot(
                primary_image_, primary_image_bytes_, engine_capability_,
                engine_slot::object_type_by_index, identity_capability,
                object_type_count - 1);
          }
          std::int32_t reflected_type_id = -1;
          if (status == RegistrationObserverError::ok && identity_capability != 0) {
            status = invoke_slot(
                primary_image_, primary_image_bytes_, identity_capability,
                type_info_slot::type_id, reflected_type_id);
          }
          if (status != RegistrationObserverError::ok || reflected_type_id <= 0) {
            return status == RegistrationObserverError::ok
                       ? RegistrationObserverError::unresolved_identity
                       : status;
          }
          engine_type_id = reflected_type_id;
        } else if (eax_result <= 0) {
          const auto& lookup_declaration =
              entry.kind == RegistrationEntryJsonKind::type_alias ? entry.name
                                                                   : entry.declaration;
          std::int32_t reflected_type_id = -1;
          status = invoke_slot(
              primary_image_, primary_image_bytes_, engine_capability_,
              engine_slot::type_id_by_declaration, reflected_type_id,
              lookup_declaration.c_str());
          if (status != RegistrationObserverError::ok || reflected_type_id <= 0 ||
              (eax_result > 0 && eax_result != reflected_type_id)) {
            return status == RegistrationObserverError::ok
                       ? RegistrationObserverError::unresolved_identity
                       : status;
          }
          engine_type_id = reflected_type_id;
        }
#endif
        live_capture_note_observer_stage_v1(0x302u);
        result.post_result.value = static_cast<std::uint32_t>(engine_type_id);
        std::uintptr_t reflected_capability = 0;
        live_capture_note_observer_stage_v1(0x303u);
        status = type_by_id(engine_type_id, reflected_capability);
        if (status != RegistrationObserverError::ok ||
            (identity_capability != 0 && identity_capability != reflected_capability)) {
          return RegistrationObserverError::type_reflection_failed;
        }
        identity_capability = reflected_capability;
        live_capture_note_observer_stage_v1(0x304u);
        status = serialization_error(correlation_.register_type(
            identity_capability, static_cast<std::uint32_t>(engine_type_id), entry.trace_id));
        if (status != RegistrationObserverError::ok) {
          return RegistrationObserverError::type_correlation_failed;
        }
        live_capture_note_observer_stage_v1(0x305u);
        TargetTypeOperationsProjection operations{};
        bool available = false;
        std::uint32_t public_alignment = 1;
        if (!funcdef && entry.kind != RegistrationEntryJsonKind::type_alias) {
          // FAngelscriptType publication is interleaved after many public type
          // registrations. Calling FromTypeId here repeatedly scans a partial
          // host table and is both premature and quadratically expensive.
          // Container identity is declaration-exact; every fixed operation is
          // projected once from the complete table at finalize_registry().
          const auto classified = classify_target_type_operations_v1(
              entry.declaration.data(), entry.declaration.size());
          if (classified == TypeOperationsJsonKind::t_array ||
              classified == TypeOperationsJsonKind::t_map ||
              classified == TypeOperationsJsonKind::t_set ||
              classified == TypeOperationsJsonKind::t_optional) {
            operations.kind = classified;
            available = true;
          }
          entry.type_operations = available ? operations.kind
                                            : TypeOperationsJsonKind::unavailable;
          entry.fixed_operations = operations.fixed;
        }
        if (object) {
          live_capture_note_observer_stage_v1(0x306u);
          const auto byte_size = signed_scalar(arguments[1]);
          std::uint32_t alignment = 0;
          if (byte_size < 0 ||
              static_cast<std::uint32_t>(byte_size) > kMaximumObjectBytes ||
              !read_value(
                  identity_capability,
                  layout_v23300::target_confirmed::object_type_alignment,
                  alignment)) {
            return RegistrationObserverError::object_type_layout_failed;
          }
          live_capture_note_type_layout_v1(
              alignment, operations.fixed.value_alignment, available);
          if (alignment == 0 || alignment > 4096 ||
              (alignment & (alignment - 1)) != 0) {
            return RegistrationObserverError::object_type_alignment_invalid;
          }
          entry.byte_size = static_cast<std::uint32_t>(byte_size);
          entry.alignment = alignment;
          public_alignment = alignment;
          entry.flags = static_cast<std::uint32_t>(arguments[2].scalar) &
                        kPublicObjectFlagMask;
          identities_.push_back({IdentityKind::object_type, identity_capability, 0, 0,
                                 entry.trace_id, static_cast<std::uint32_t>(engine_type_id), 0,
                                 entry.byte_size, entry.flags});
        }
        live_capture_note_observer_stage_v1(0x307u);
        types_.push_back({identity_capability, entry.trace_id,
                          static_cast<std::uint32_t>(engine_type_id), entry.byte_size,
                          public_alignment, entry.flags, entry.type_operations,
                          entry.declaration, entry.context.name_space,
                          !funcdef && entry.kind != RegistrationEntryJsonKind::type_alias});
        if (!add_count(expected_delta_.types) ||
            (funcdef && !add_count(expected_delta_.funcdefs)) ||
            (entry.kind == RegistrationEntryJsonKind::type_alias &&
             !add_count(expected_delta_.typedefs))) {
          return RegistrationObserverError::limit_exceeded;
        }
        live_capture_note_observer_stage_v1(0x308u);
        break;
      }
      case GORE_AS_CAPTURE_REGISTRATION_INTERFACE_METHOD_V1:
        entry.kind = RegistrationEntryJsonKind::interface_method;
        status = resolve_owner(0);
        entry.declaration = text(arguments[1]);
        if (status == RegistrationObserverError::ok) status = resolve_function();
        result.has_owner_engine_type_id = status == RegistrationObserverError::ok;
        result.owner_engine_type_id = owner_engine_id;
        if (status == RegistrationObserverError::ok && !add_count(expected_delta_.functions)) {
          status = RegistrationObserverError::limit_exceeded;
        }
        break;
      case GORE_AS_CAPTURE_REGISTRATION_OBJECT_PROPERTY_V1:
        entry.kind = RegistrationEntryJsonKind::object_property;
        if (eax_result != 0) return RegistrationObserverError::result_rejected;
        status = resolve_owner(0);
        entry.declaration = text(arguments[1]);
        entry.byte_offset = static_cast<std::uint32_t>(signed_scalar(arguments[2]));
        entry.composite_offset = static_cast<std::uint32_t>(signed_scalar(arguments[3]));
        entry.is_composite_indirect = arguments[4].scalar != 0;
        entry.accessor_type = static_cast<std::uint32_t>(arguments[5].scalar);
        entry.is_protected = arguments[6].scalar != 0;
        if (signed_scalar(arguments[2]) < 0 || signed_scalar(arguments[3]) < 0 ||
            entry.byte_offset > kMaximumOffset ||
            entry.composite_offset > kMaximumOffset ||
            entry.accessor_type > 0xff) {
          return RegistrationObserverError::invalid_raw_frame;
        }
        if (status == RegistrationObserverError::ok) {
          std::uint32_t property_index = 0;
          status = latest_object_property(
              owner_capability, property_index, identity_capability);
          if (status != RegistrationObserverError::ok) {
            return RegistrationObserverError::object_property_lookup_failed;
          }
          status = serialization_error(correlation_.register_object_property(
              owner_capability, property_index, entry.trace_id,
              owner_engine_id));
          if (status != RegistrationObserverError::ok) {
            return RegistrationObserverError::object_property_correlation_failed;
          }
          result.post_result.value = property_index;
          result.has_owner_engine_type_id = true;
          result.owner_engine_type_id = owner_engine_id;
          identities_.push_back({IdentityKind::object_property, identity_capability,
                                 owner_capability, 0, entry.trace_id, owner_engine_id,
                                 property_index, 0, 0});
          if (!add_count(expected_delta_.object_properties)) {
            status = RegistrationObserverError::limit_exceeded;
          }
        }
        break;
      case GORE_AS_CAPTURE_REGISTRATION_OBJECT_METHOD_V1:
      case GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1: {
        const bool behaviour =
            pending.raw.kind == GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1;
        entry.kind = behaviour ? RegistrationEntryJsonKind::object_behaviour
                               : RegistrationEntryJsonKind::object_method;
        status = resolve_owner(0);
        entry.declaration = text(arguments[behaviour ? 2 : 1]);
        if (behaviour) {
          const auto behaviour_id = static_cast<std::uint32_t>(arguments[1].scalar);
          if (behaviour_id >= kBehaviours.size()) {
            return RegistrationObserverError::invalid_raw_frame;
          }
          entry.behaviour = kBehaviours[behaviour_id];
          if (behaviour_id == 8) {
            const auto* adapter = template_adapter(text(arguments[0]));
            if (adapter == nullptr || !container_adapter_matches(owner_operations, adapter)) {
              return RegistrationObserverError::type_operations_rejected;
            }
            entry.has_template_validation_adapter = true;
            entry.template_validation_adapter = adapter;
          }
          entry.composite_offset =
              static_cast<std::uint32_t>(signed_scalar(arguments[7]));
          entry.is_composite_indirect = arguments[8].scalar != 0;
          if (signed_scalar(arguments[7]) < 0 ||
              entry.composite_offset > kMaximumOffset) {
            return RegistrationObserverError::invalid_raw_frame;
          }
          if (status == RegistrationObserverError::ok) {
            status = callable_stubs(3, 4, 5, 6);
          }
        } else {
          entry.composite_offset =
              static_cast<std::uint32_t>(signed_scalar(arguments[6]));
          entry.is_composite_indirect = arguments[7].scalar != 0;
          entry.accessor_type = static_cast<std::uint32_t>(arguments[8].scalar);
          if (signed_scalar(arguments[6]) < 0 ||
              entry.composite_offset > kMaximumOffset || entry.accessor_type > 0xff) {
            return RegistrationObserverError::invalid_raw_frame;
          }
          if (status == RegistrationObserverError::ok) {
            status = callable_stubs(2, 3, 4, 5);
          }
        }
        if (status == RegistrationObserverError::ok) status = resolve_function();
        result.has_owner_engine_type_id = status == RegistrationObserverError::ok;
        result.owner_engine_type_id = owner_engine_id;
        if (status == RegistrationObserverError::ok && !add_count(expected_delta_.functions)) {
          status = RegistrationObserverError::limit_exceeded;
        }
        break;
      }
      case GORE_AS_CAPTURE_REGISTRATION_STRING_FACTORY_V1:
        entry.kind = RegistrationEntryJsonKind::string_factory;
        entry.declaration = text(arguments[0]);
        stubs.has_factory_object = true;
        status = object_pointer_token(
            arguments[1].pointer_capability, stubs.factory_object_pointer_token);
        break;
      case GORE_AS_CAPTURE_REGISTRATION_DEFAULT_ARRAY_TYPE_V1:
        entry.kind = RegistrationEntryJsonKind::default_array_type;
        entry.declaration = text(arguments[0]);
        break;
      case GORE_AS_CAPTURE_REGISTRATION_ENUM_VALUE_V1:
        entry.kind = RegistrationEntryJsonKind::enum_value;
        if (eax_result != 0) return RegistrationObserverError::result_rejected;
        status = resolve_owner(0);
        entry.name = text(arguments[1]);
        entry.enum_value = signed_scalar(arguments[2]);
        result.has_owner_engine_type_id = status == RegistrationObserverError::ok;
        result.owner_engine_type_id = owner_engine_id;
#if !defined(GORE_AS_CAPTURE_TEST_TARGET)
        if (status == RegistrationObserverError::ok) {
          std::uint32_t value_count = 0;
          status = invoke_slot(
              primary_image_, primary_image_bytes_, owner_capability,
              type_info_slot::enum_value_count, value_count);
          if (status != RegistrationObserverError::ok || value_count == 0 ||
              value_count > kMaximumItems) {
            return status == RegistrationObserverError::ok
                       ? RegistrationObserverError::unresolved_identity
                       : status;
          }
          const auto value_index = value_count - 1;
          const char* reflected_name = nullptr;
          std::int32_t reflected_value = 0;
          status = invoke_slot(
              primary_image_, primary_image_bytes_, owner_capability,
              type_info_slot::enum_value_by_index, reflected_name, value_index,
              &reflected_value);
          if (status != RegistrationObserverError::ok ||
              !cstring_equals(reflected_name, entry.name) ||
              reflected_value != entry.enum_value) {
            return status == RegistrationObserverError::ok
                       ? RegistrationObserverError::unresolved_identity
                       : status;
          }
          result.post_result.value = value_index;
        }
#else
        if (status == RegistrationObserverError::ok) {
          result.post_result.value = static_cast<std::uint32_t>(std::count_if(
              completed_records_.begin(), completed_records_.end(),
              [&](const auto& record) {
                return record.entry.kind == RegistrationEntryJsonKind::enum_value &&
                       record.entry.owner_trace_type_id == entry.owner_trace_type_id;
              }));
        }
#endif
        if (status == RegistrationObserverError::ok && !add_count(expected_delta_.enum_values)) {
          status = RegistrationObserverError::limit_exceeded;
        }
        break;
      default:
        return RegistrationObserverError::invalid_raw_frame;
    }
    if (status != RegistrationObserverError::ok) return status;
    live_capture_note_observer_stage_v1(0x400u);
    status = serialization_error(host_stubs_.derive_registration_stubs(entry, stubs));
    if (status != RegistrationObserverError::ok) {
      return RegistrationObserverError::host_stub_derivation_failed;
    }
    live_capture_note_observer_stage_v1(0x401u);
    status = serialization_error(final_sequence_.observe_registration(entry));
    if (status != RegistrationObserverError::ok) {
      return RegistrationObserverError::final_sequence_failed;
    }
    if (!add_count(expected_delta_.total_registrations)) {
      return RegistrationObserverError::limit_exceeded;
    }
    CompletedRegistrationProjection value{};
    value.entry = entry;
    value.result = result;
    live_capture_note_observer_stage_v1(0x402u);
    status = serialization_error(serialize_registry_delta_json_v1(
        bind_callback_ordinal, value.entry, value.result, value.delta_json));
    if (status != RegistrationObserverError::ok) return status;
    live_capture_note_observer_stage_v1(0x403u);
    completed_records_.push_back(
        {bind_callback_ordinal, value.entry, value.result, global_storage_type_id});
    completed = std::move(value);
    return RegistrationObserverError::ok;
  } catch (...) {
    return RegistrationObserverError::limit_exceeded;
  }
}

RegistrationObserverError TargetRegistrationObserver::finalize_registry(
    const std::uint32_t bind_count,
    std::vector<std::vector<std::string>>& replacement_deltas,
    std::string& support_json) noexcept {
  if (!begun_ || finalized_ || bind_count == 0 || completed_records_.empty()) {
    return RegistrationObserverError::invalid_state;
  }
  try {
    // FAngelscriptType registration is intentionally interleaved with the
    // public Register* stream.  A bind often installs a value object and its
    // global constants before publishing the host TypeUsage.  Re-project once
    // at the final bind boundary and upgrade the buffered type entries.
    for (auto& type : types_) {
      if (!type.refresh_operations) continue;
      TargetTypeOperationsProjection operations{};
      bool available = false;
      auto status = resolve_type_projection_cached(
          static_cast<std::int32_t>(type.engine_id), type.declaration,
          operations, available);
      if (status != RegistrationObserverError::ok) return status;
      if (!available) continue;
      type.operations = operations.kind;
      const auto record = std::find_if(
          completed_records_.begin(), completed_records_.end(),
          [&](const auto& value) {
            return value.entry.trace_id == type.trace_id &&
                   (value.entry.kind == RegistrationEntryJsonKind::object_type ||
                    value.entry.kind == RegistrationEntryJsonKind::interface ||
                    value.entry.kind == RegistrationEntryJsonKind::enumeration);
          });
      if (record == completed_records_.end()) {
        return RegistrationObserverError::unresolved_identity;
      }
      record->entry.type_operations = operations.kind;
      record->entry.fixed_operations = operations.fixed;
    }

    // Upgrade every storage stub from the complete TypeUsage table.  If a
    // target type deliberately has no host TypeUsage, the safe extent captured
    // from RegisterObjectType remains authoritative.
    for (auto& record : completed_records_) {
      if (record.entry.kind != RegistrationEntryJsonKind::global_property ||
          record.global_storage_type_id < 0) {
        continue;
      }
      std::uint32_t primitive_size = 0;
      std::uint32_t primitive_alignment = 0;
      if (primitive_storage_layout(
              record.global_storage_type_id, primitive_size,
              primitive_alignment)) {
        continue;
      }
      TargetTypeOperationsProjection operations{};
      const auto cached = std::find_if(
          type_projection_cache_.begin(), type_projection_cache_.end(),
          [&](const TypeProjectionRecord& value) {
            return value.engine_type_id == record.global_storage_type_id;
          });
      if (cached == type_projection_cache_.end()) continue;
      operations = cached->projection;
      if (operations.fixed.value_size == 0 ||
          operations.fixed.value_alignment == 0) {
        return RegistrationObserverError::global_property_zero_value_size;
      }
      auto status = serialization_error(host_stubs_.update_storage_descriptor(
          record.entry.storage_stub_id, operations.fixed.value_size,
          operations.fixed.value_alignment));
      if (status != RegistrationObserverError::ok) return status;
    }

    std::vector<std::vector<std::string>> deltas(bind_count);
    for (const auto& record : completed_records_) {
      if (record.bind_ordinal >= bind_count) {
        return RegistrationObserverError::serialization_rejected;
      }
      std::string json;
      const auto status = serialization_error(serialize_registry_delta_json_v1(
          record.bind_ordinal, record.entry, record.result, json));
      if (status != RegistrationObserverError::ok) return status;
      deltas[record.bind_ordinal].push_back(std::move(json));
    }
    std::string support;
    auto status = serialization_error(
        serialize_registry_support_json_v1(host_stubs_, support));
    if (status != RegistrationObserverError::ok) return status;
    replacement_deltas = std::move(deltas);
    support_json = std::move(support);
    return RegistrationObserverError::ok;
  } catch (...) {
    return RegistrationObserverError::limit_exceeded;
  }
}

bool TargetRegistrationObserver::expected_counts_match(
    const RegistryCounts& final_counts) const noexcept {
#define GORE_AS_COUNT_MATCH(field)                                                \
  (baseline_.field <= std::numeric_limits<std::uint32_t>::max() -              \
                          expected_delta_.field &&                              \
   baseline_.field + expected_delta_.field == final_counts.field)
  return GORE_AS_COUNT_MATCH(types) && GORE_AS_COUNT_MATCH(functions) &&
         GORE_AS_COUNT_MATCH(object_properties) &&
         GORE_AS_COUNT_MATCH(global_properties) && GORE_AS_COUNT_MATCH(enum_values) &&
         GORE_AS_COUNT_MATCH(funcdefs) && GORE_AS_COUNT_MATCH(typedefs) &&
         GORE_AS_COUNT_MATCH(total_registrations);
#undef GORE_AS_COUNT_MATCH
}

RegistrationObserverError TargetRegistrationObserver::projected_counts(
    RegistryCounts& counts) const noexcept {
  if (!begun_ || finalized_) return RegistrationObserverError::invalid_state;
  RegistryCounts projected{};
#define GORE_AS_PROJECT_COUNT(field)                                             \
  do {                                                                           \
    if (baseline_.field > std::numeric_limits<std::uint32_t>::max() -            \
                              expected_delta_.field) {                            \
      return RegistrationObserverError::limit_exceeded;                          \
    }                                                                            \
    projected.field = baseline_.field + expected_delta_.field;                   \
  } while (false)
  GORE_AS_PROJECT_COUNT(types);
  GORE_AS_PROJECT_COUNT(functions);
  GORE_AS_PROJECT_COUNT(object_properties);
  GORE_AS_PROJECT_COUNT(global_properties);
  GORE_AS_PROJECT_COUNT(enum_values);
  GORE_AS_PROJECT_COUNT(funcdefs);
  GORE_AS_PROJECT_COUNT(typedefs);
  GORE_AS_PROJECT_COUNT(total_registrations);
#undef GORE_AS_PROJECT_COUNT
  counts = projected;
  return RegistrationObserverError::ok;
}

RegistrationObserverError TargetRegistrationObserver::enumerate_post_bind_final_state(
    const RegistryCounts& final_counts,
    std::vector<std::string>& json_records) noexcept {
  if (!begun_ || finalized_) return RegistrationObserverError::invalid_state;
  try {
    auto sequence = final_sequence_;
    auto status = serialization_error(sequence.begin_final_state());
    if (status != RegistrationObserverError::ok) return status;
    std::vector<std::string> records;
    records.reserve(identities_.size());
    TypeIdResolver resolver{};
    resolver.context = &correlation_;
    resolver.resolve = &resolve_trace_type;
    for (const auto& identity : identities_) {
      live_capture_note_reflected_type_v1(
          static_cast<std::int32_t>(identity.engine_id),
          0x100u + static_cast<std::uint32_t>(identity.kind),
          identity.trace_id, 0, true);
      std::uintptr_t current = 0;
      std::string json;
      switch (identity.kind) {
        case IdentityKind::object_type: {
          status = type_by_id(static_cast<std::int32_t>(identity.engine_id), current);
          if (status != RegistrationObserverError::ok || current != identity.capability) {
            return RegistrationObserverError::unresolved_identity;
          }
          ObjectTypeFinalState state{};
          if (extract_object_type_final_state_v23300(
                  identity.capability, identity.trace_id, identity.public_byte_size,
                  identity.public_flags, resolver, state) != FinalStateError::ok) {
            return RegistrationObserverError::unreadable_target;
          }
          status = serialization_error(sequence.append(state, json));
          break;
        }
        case IdentityKind::object_property: {
          status = object_property(
              identity.owner_capability, identity.member_index, current);
          if (status != RegistrationObserverError::ok || current != identity.capability) {
            return RegistrationObserverError::unresolved_identity;
          }
          ObjectPropertyFinalState state{};
          if (extract_object_property_final_state_v23300(
                  identity.capability, identity.trace_id, state) != FinalStateError::ok) {
            return RegistrationObserverError::unreadable_target;
          }
          status = serialization_error(sequence.append(state, json));
          break;
        }
        case IdentityKind::function: {
          status = function_by_id(static_cast<std::int32_t>(identity.engine_id), current);
          if (status != RegistrationObserverError::ok || current != identity.capability) {
            return RegistrationObserverError::unresolved_identity;
          }
          FunctionFinalState state{};
          const auto final_state_status = extract_function_final_state_v23300(
              identity.capability, identity.trace_id, state);
          if (final_state_status != FinalStateError::ok) {
            const auto source = std::find_if(
                completed_records_.begin(), completed_records_.end(),
                [&](const CompletedRecord& record) {
                  const auto kind = record.entry.kind;
                  const bool function =
                      kind == RegistrationEntryJsonKind::global_function ||
                      kind == RegistrationEntryJsonKind::interface_method ||
                      kind == RegistrationEntryJsonKind::object_method ||
                      kind == RegistrationEntryJsonKind::object_behaviour;
                  return function && record.entry.trace_id == identity.trace_id;
                });
            if (source != completed_records_.end()) {
              std::string_view owner;
              if (source->result.has_owner_engine_type_id) {
                const auto owner_type = std::find_if(
                    types_.begin(), types_.end(), [&](const TypeRecord& type) {
                      return type.engine_id == source->result.owner_engine_type_id;
                    });
                if (owner_type != types_.end()) owner = owner_type->declaration;
              }
              std::uint32_t traits = 0;
              std::uint32_t exposed_type = 0;
              std::int8_t hidden_index = -1;
              std::int8_t output_index = -1;
              std::uint32_t compile_out = 0;
              std::uintptr_t system_interface = 0;
              (void)read_value(
                  identity.capability, layout_v23300::donor::script_function_traits,
                  traits);
              (void)read_value(
                  identity.capability, layout_v23300::donor::script_function_exposed_type,
                  exposed_type);
              (void)read_value(
                  identity.capability,
                  layout_v23300::donor::script_function_hidden_argument_index,
                  hidden_index);
              (void)read_value(
                  identity.capability,
                  layout_v23300::donor::script_function_output_type_argument_index,
                  output_index);
              (void)read_value(
                  identity.capability,
                  layout_v23300::donor::script_function_compile_out_type, compile_out);
              (void)read_value(
                  identity.capability,
                  layout_v23300::donor::script_function_system_interface,
                  system_interface);
              live_capture_note_registration_arguments_v1(
                  source->entry.declaration.data(),
                  static_cast<std::uint32_t>(source->entry.declaration.size()),
                  owner.data(), static_cast<std::uint32_t>(owner.size()), traits,
                  static_cast<std::uint64_t>(exposed_type) |
                      (static_cast<std::uint64_t>(static_cast<std::uint8_t>(hidden_index))
                       << 32) |
                      (static_cast<std::uint64_t>(static_cast<std::uint8_t>(output_index))
                       << 40) |
                      (static_cast<std::uint64_t>(compile_out) << 48),
                  system_interface);
            }
            live_capture_note_failure_detail_v1(
                0xA100u + static_cast<std::uint32_t>(final_state_status));
            return RegistrationObserverError::unreadable_target;
          }
          status = serialization_error(sequence.append(state, json));
          break;
        }
        case IdentityKind::global_property: {
          std::int32_t type_id = 0;
          std::uintptr_t storage = 0;
          status = global_property(identity.member_index, type_id, storage, current);
          if (status != RegistrationObserverError::ok || current != identity.capability ||
              storage != identity.storage_capability) {
            return RegistrationObserverError::unresolved_identity;
          }
          GlobalPropertyFinalState state{};
          if (extract_global_property_final_state_v23300(
                  identity.capability, identity.trace_id, state) != FinalStateError::ok) {
            return RegistrationObserverError::unreadable_target;
          }
          status = serialization_error(sequence.append(state, json));
          break;
        }
      }
      if (status != RegistrationObserverError::ok) return status;
      records.push_back(std::move(json));
    }
    if (!sequence.complete()) return RegistrationObserverError::unresolved_identity;
    if (!expected_counts_match(final_counts)) {
      return RegistrationObserverError::registry_count_drift;
    }
    final_sequence_ = std::move(sequence);
    finalized_ = true;
    json_records = std::move(records);
    return RegistrationObserverError::ok;
  } catch (...) {
    return RegistrationObserverError::limit_exceeded;
  }
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

struct ObserverFixture final {
  std::array<std::byte, 0x1700> engine{};
  std::array<std::byte, 24> name_space{};
  std::array<std::array<std::byte, 0x2d8>, 5> types{};
  std::array<std::array<std::byte, 0x188>, 4> functions{};
  std::array<std::byte, 0x50> object_property{};
  std::array<std::byte, 0x70> global_property{};
  std::uintptr_t storage{0x7000};
  bool storage_drift{};
};

template <typename Value>
void fixture_write(std::byte* object, const std::size_t offset, const Value& value) noexcept {
  std::memcpy(object + offset, &value, sizeof(value));
}

bool fixture_token(void*, const std::uintptr_t capability, std::uint32_t& token) noexcept {
  if (capability == 0) return false;
  token = static_cast<std::uint32_t>((capability >> 4) & 0x7fff'ffffu);
  return token != std::numeric_limits<std::uint32_t>::max();
}

bool fixture_type_by_id(
    void* context,
    const std::int32_t id,
    std::uintptr_t& capability) noexcept {
  auto* fixture = static_cast<ObserverFixture*>(context);
  constexpr std::array ids{10, 20, 30, 40, 50};
  const auto found = std::find(ids.begin(), ids.end(), id);
  if (fixture == nullptr || found == ids.end()) return false;
  capability = reinterpret_cast<std::uintptr_t>(
      fixture->types[static_cast<std::size_t>(found - ids.begin())].data());
  return true;
}

bool fixture_function_by_id(
    void* context,
    const std::int32_t id,
    std::uintptr_t& capability) noexcept {
  auto* fixture = static_cast<ObserverFixture*>(context);
  if (fixture == nullptr || id < 100 || id > 103) return false;
  capability = reinterpret_cast<std::uintptr_t>(fixture->functions[id - 100].data());
  return true;
}

bool fixture_type_by_declaration(
    void* context,
    const char* declaration,
    std::uintptr_t& capability) noexcept {
  if (declaration == nullptr) return false;
  const std::string_view value(declaration);
  const std::int32_t id = value == "TArray<class T>" ? 10 : value == "I" ? 20
                               : value == "E"          ? 30
                                                        : -1;
  return id >= 0 && fixture_type_by_id(context, id, capability);
}

bool fixture_global_property(
    void* context,
    const std::uint32_t index,
    std::int32_t& type_id,
    std::uintptr_t& storage,
    std::uintptr_t& property) noexcept {
  auto* fixture = static_cast<ObserverFixture*>(context);
  if (fixture == nullptr || index != 0) return false;
  type_id = 4;
  storage = fixture->storage + (fixture->storage_drift ? 8 : 0);
  property = reinterpret_cast<std::uintptr_t>(fixture->global_property.data());
  return true;
}

bool fixture_object_property(
    void* context,
    const std::uintptr_t owner,
    const std::uint32_t index,
    std::uintptr_t& property) noexcept {
  auto* fixture = static_cast<ObserverFixture*>(context);
  if (fixture == nullptr || index != 0 ||
      owner != reinterpret_cast<std::uintptr_t>(fixture->types[0].data())) {
    return false;
  }
  property = reinterpret_cast<std::uintptr_t>(fixture->object_property.data());
  return true;
}

bool fixture_type_operations(
    void*,
    const std::int32_t,
    const char* declaration,
    TargetTypeOperationsProjection& projection) noexcept {
  if (declaration == nullptr) return false;
  projection = {};
  projection.kind = std::string_view(declaration).starts_with("TArray<")
                        ? TypeOperationsJsonKind::t_array
                        : TypeOperationsJsonKind::fixed;
  projection.subtype_count = projection.kind == TypeOperationsJsonKind::t_array ? 1 : 0;
  projection.fixed.can_create_property = true;
  projection.fixed.never_requires_gc = false;
  projection.fixed.requires_property = false;
  projection.fixed.can_be_template_subtype = true;
  projection.fixed.can_construct = true;
  projection.fixed.can_destruct = true;
  projection.fixed.can_copy = true;
  projection.fixed.can_compare = true;
  projection.fixed.can_hash_value = true;
  projection.fixed.value_size = 8;
  // AngelScript's object-type layout and the host value-operation alignment
  // are independent contracts.  Exercise the target-observed 8/4 split.
  projection.fixed.value_alignment = 4;
  return true;
}

const registration::RegistrationHookPoint* fixture_hook(const std::uint32_t kind) noexcept {
  const auto found = std::find_if(
      registration::kPinnedRegistrationHooks.begin(),
      registration::kPinnedRegistrationHooks.end(),
      [&](const auto& value) { return value.kind == kind; });
  return found == registration::kPinnedRegistrationHooks.end() ? nullptr : &*found;
}

RawRegistrationEntry fixture_raw(
    ObserverFixture& fixture,
    const std::uint32_t kind) noexcept {
  RawRegistrationEntry raw{};
  const auto* hook = fixture_hook(kind);
  if (hook == nullptr) return raw;
  raw.kind = kind;
  raw.engine_capability = reinterpret_cast<std::uintptr_t>(fixture.engine.data());
  raw.argument_count = hook->argument_count;
  for (std::size_t index = 0; index < raw.argument_count; ++index) {
    raw.arguments[index].semantic = hook->arguments[index].semantic;
    if (raw.arguments[index].semantic == GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1) {
      raw.arguments[index].opaque_descriptor_bytes =
          layout_v23300::donor::function_pointer_descriptor_bytes;
      const std::uintptr_t function = 0x1000 + kind * 0x10;
      std::memcpy(raw.arguments[index].opaque_descriptor.data(), &function, sizeof(function));
      raw.arguments[index].opaque_descriptor[
          layout_v23300::donor::function_pointer_descriptor_flag] = std::byte{2};
    } else if (raw.arguments[index].semantic ==
               GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1) {
      raw.arguments[index].opaque_descriptor_bytes =
          layout_v23300::donor::function_caller_descriptor_bytes;
    }
  }
  return raw;
}

void fixture_text(RawRegistrationEntry& raw, const std::size_t index, const char* value) {
  const auto bytes = std::strlen(value);
  std::memcpy(raw.arguments[index].text.data(), value, bytes + 1);
  raw.arguments[index].text_bytes = static_cast<std::uint32_t>(bytes);
}

bool fixture_prepare(ObserverFixture& fixture) noexcept {
  const std::uint32_t mask = 0x55aa;
  fixture_write(
      fixture.engine.data(),
      layout_v23300::target_confirmed::target_engine_default_access_mask,
      mask);
  const std::uintptr_t name_space =
      reinterpret_cast<std::uintptr_t>(fixture.name_space.data());
  fixture_write(
      fixture.engine.data(),
      layout_v23300::target_confirmed::target_engine_default_namespace,
      name_space);
  const std::uint32_t name_space_bytes = 4;
  fixture_write(fixture.name_space.data(), 0, name_space_bytes);
  std::memcpy(fixture.name_space.data() + 8, "Game", 5);
  for (auto& type : fixture.types) {
    const std::uint32_t alignment = 8;
    fixture_write(
        type.data(), layout_v23300::target_confirmed::object_type_alignment, alignment);
  }
  for (auto& function : fixture.functions) {
    const std::int8_t absent = -1;
    fixture_write(
        function.data(),
        layout_v23300::donor::script_function_hidden_argument_index,
        absent);
    fixture_write(
        function.data(),
        layout_v23300::donor::script_function_output_type_argument_index,
        absent);
  }
  return true;
}

}  // namespace

bool target_registration_observer_selftest_v1() noexcept {
  ObserverFixture fixture{};
  if (!fixture_prepare(fixture)) return false;
  RegistrationObserverTestTarget target{};
  target.context = &fixture;
  target.type_by_id = &fixture_type_by_id;
  target.function_by_id = &fixture_function_by_id;
  target.type_by_declaration = &fixture_type_by_declaration;
  target.global_property = &fixture_global_property;
  target.object_property = &fixture_object_property;
  target.type_operations = &fixture_type_operations;
  PointerTokenResolver tokens{};
  tokens.resolve = &fixture_token;
  tokens.resolve_object = &fixture_token;
  tokens.resolve_storage = &fixture_token;
  TargetRegistrationObserver observer(
      1, 1, reinterpret_cast<std::uintptr_t>(fixture.engine.data()), tokens, target);
  RegistryCounts baseline{};
  if (observer.begin_observation(baseline) != RegistrationObserverError::ok) return false;

  struct Case final { std::uint32_t kind; std::int32_t eax; };
  constexpr std::array cases{
      Case{1, 100}, Case{2, 0}, Case{3, 10}, Case{4, 0}, Case{5, 101},
      Case{6, 102}, Case{7, 20}, Case{8, 103}, Case{9, 0}, Case{10, 0},
      Case{11, 30}, Case{12, 0}, Case{13, 40}, Case{14, 50},
  };
  for (const auto& value : cases) {
    auto raw = fixture_raw(fixture, value.kind);
    switch (value.kind) {
      case 1:
        fixture_text(raw, 0, "void Global()");
        raw.arguments[2].scalar = 0;
        break;
      case 2:
        fixture_text(raw, 0, "int Value");
        raw.arguments[1].pointer_capability = fixture.storage;
        break;
      case 3:
        fixture_text(raw, 0, "TArray<class T>");
        raw.arguments[1].scalar = 8;
        raw.arguments[2].scalar = 0x8000'0042u;
        break;
      case 4:
        fixture_text(raw, 0, "TArray<class T>");
        fixture_text(raw, 1, "int Field");
        raw.arguments[5].scalar = 255;
        break;
      case 5:
        fixture_text(raw, 0, "TArray<class T>");
        fixture_text(raw, 1, "void Method()");
        raw.arguments[3].scalar = 0;
        raw.arguments[8].scalar = 255;
        break;
      case 6:
        fixture_text(raw, 0, "TArray<class T>");
        raw.arguments[1].scalar = 8;
        fixture_text(raw, 2, "bool Validate(int&in)");
        raw.arguments[4].scalar = 0;
        break;
      case 7:
        fixture_text(raw, 0, "I");
        break;
      case 8:
        fixture_text(raw, 0, "I");
        fixture_text(raw, 1, "void F()");
        break;
      case 9:
        fixture_text(raw, 0, "FString");
        raw.arguments[1].pointer_capability = 0x6000;
        break;
      case 10:
        fixture_text(raw, 0, "TArray<T>");
        break;
      case 11:
        fixture_text(raw, 0, "E");
        break;
      case 12:
        fixture_text(raw, 0, "E");
        fixture_text(raw, 1, "One");
        raw.arguments[2].scalar = 1;
        break;
      case 13:
        fixture_text(raw, 0, "void Callback()");
        break;
      case 14:
        fixture_text(raw, 0, "Alias");
        fixture_text(raw, 1, "uint");
        break;
      default:
        return false;
    }
    PendingRegistrationProjection pending{};
    CompletedRegistrationProjection completed{};
    if (observer.prepare(raw, pending) != RegistrationObserverError::ok ||
        observer.complete(7, pending, value.eax, completed) !=
            RegistrationObserverError::ok ||
        completed.delta_json.empty() || completed.entry.ordinal + 1 != value.kind) {
      return false;
    }
    if (value.kind == 3 &&
        (completed.entry.flags != 0x42 ||
         completed.entry.type_operations != TypeOperationsJsonKind::t_array)) {
      return false;
    }
    if (value.kind == 6 &&
        (!completed.entry.has_template_validation_adapter ||
         completed.entry.template_validation_adapter != "t_array")) {
      return false;
    }
  }
  std::string support;
  std::vector<std::vector<std::string>> replacement_deltas;
  if (observer.finalize_registry(8, replacement_deltas, support) !=
          RegistrationObserverError::ok ||
      support.empty() || replacement_deltas.size() != 8 ||
      replacement_deltas[7].size() != cases.size()) {
    return false;
  }
  RegistryCounts final_counts{};
  final_counts.types = 5;
  final_counts.functions = 4;
  final_counts.object_properties = 1;
  final_counts.global_properties = 1;
  final_counts.enum_values = 1;
  final_counts.funcdefs = 1;
  final_counts.typedefs = 1;
  final_counts.total_registrations = 14;
  auto drift = final_counts;
  ++drift.functions;
  std::vector<std::string> records;
  if (observer.enumerate_post_bind_final_state(drift, records) !=
          RegistrationObserverError::registry_count_drift ||
      observer.enumerate_post_bind_final_state(final_counts, records) !=
          RegistrationObserverError::ok ||
      records.size() != 7) {
    return false;
  }

  ObserverFixture mismatch{};
  if (!fixture_prepare(mismatch)) return false;
  mismatch.storage_drift = true;
  target.context = &mismatch;
  TargetRegistrationObserver rejected(
      1, 1, reinterpret_cast<std::uintptr_t>(mismatch.engine.data()), tokens, target);
  if (rejected.begin_observation({}) != RegistrationObserverError::ok) return false;
  auto raw = fixture_raw(mismatch, GORE_AS_CAPTURE_REGISTRATION_GLOBAL_PROPERTY_V1);
  fixture_text(raw, 0, "int Value");
  raw.arguments[1].pointer_capability = mismatch.storage;
  PendingRegistrationProjection pending{};
  CompletedRegistrationProjection completed{};
  return rejected.prepare(raw, pending) == RegistrationObserverError::ok &&
         rejected.complete(0, pending, 0, completed) ==
             RegistrationObserverError::unresolved_identity;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
