#include "target_final_state.hpp"

#include "target_layout.hpp"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstring>
#include <limits>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

namespace layout = layout_v23300;

constexpr std::uint32_t kMaximumInterfaces = 2'000'000;
constexpr std::uint32_t kMaximumStringBytes = 64 * 1024;
constexpr std::uint32_t kMaximumAlignment = 4096;

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

struct ArrayView final {
  std::uintptr_t data{};
  std::uint32_t length{};
  std::uint32_t capacity{};
};

bool read_array(
    const std::uintptr_t object,
    const std::size_t offset,
    const std::size_t item_bytes,
    ArrayView& result) noexcept {
  ArrayView value{};
  if (!read_value(object, offset, value.data) ||
      !read_value(object, offset + sizeof(value.data), value.length) ||
      !read_value(
          object,
          offset + sizeof(value.data) + sizeof(value.length),
          value.capacity) ||
      value.length > value.capacity || value.length > kMaximumInterfaces ||
      (value.length != 0 && value.data == 0) ||
      (value.length != 0 &&
       (item_bytes > std::numeric_limits<std::size_t>::max() / value.length ||
        !readable_range(value.data, item_bytes * value.length)))) {
    return false;
  }
  result = value;
  return true;
}

bool resolve_optional_type(
    const std::uintptr_t capability,
    const TypeIdResolver resolver,
    bool& present,
    std::uint32_t& type_id) noexcept {
  present = capability != 0;
  type_id = 0;
  return capability == 0 ||
         (resolver.resolve != nullptr &&
          resolver.resolve(resolver.context, capability, type_id));
}

bool read_as_cstring(
    const std::uintptr_t string_capability,
    std::string& output) {
  std::uint32_t length = 0;
  if (!read_value(string_capability, 0, length) || length > kMaximumStringBytes) return false;
  std::uintptr_t bytes = string_capability + 8;
  if (length >= 12 && !read_value(string_capability, 8, bytes)) return false;
  if (bytes == 0 || !readable_range(bytes, static_cast<std::size_t>(length) + 1)) return false;
  const auto* text = reinterpret_cast<const char*>(bytes);
  if (text[length] != '\0' || std::memchr(text, '\0', length) != nullptr) return false;
  try {
    output.assign(text, length);
  } catch (...) {
    return false;
  }
  std::size_t cursor = 0;
  while (cursor < output.size()) {
    const auto lead = static_cast<unsigned char>(output[cursor]);
    std::size_t continuation = 0;
    std::uint32_t scalar = 0;
    if (lead < 0x80) {
      ++cursor;
      continue;
    }
    if ((lead & 0xe0) == 0xc0) {
      continuation = 1;
      scalar = lead & 0x1f;
    } else if ((lead & 0xf0) == 0xe0) {
      continuation = 2;
      scalar = lead & 0x0f;
    } else if ((lead & 0xf8) == 0xf0) {
      continuation = 3;
      scalar = lead & 0x07;
    } else {
      return false;
    }
    if (cursor + continuation >= output.size()) return false;
    for (std::size_t index = 1; index <= continuation; ++index) {
      const auto value = static_cast<unsigned char>(output[cursor + index]);
      if ((value & 0xc0) != 0x80) return false;
      scalar = (scalar << 6) | (value & 0x3f);
    }
    if ((continuation == 1 && scalar < 0x80) ||
        (continuation == 2 && scalar < 0x800) ||
        (continuation == 3 && scalar < 0x10000) || scalar > 0x10ffff ||
        (scalar >= 0xd800 && scalar <= 0xdfff)) {
      return false;
    }
    cursor += continuation + 1;
  }
  return true;
}

bool bool_byte(const std::uint8_t value) noexcept { return value <= 1; }

}  // namespace

FinalStateError extract_registration_context_v23300(
    const std::uintptr_t engine,
    RegistrationContextFinalState& result) noexcept {
  if (engine == 0) return FinalStateError::invalid_argument;
  RegistrationContextFinalState value{};
  std::uintptr_t name_space = 0;
  if (!read_value(
          engine,
          layout::target_confirmed::target_engine_default_access_mask,
          value.access_mask) ||
      !read_value(
          engine,
          layout::target_confirmed::target_engine_default_namespace,
          name_space)) {
    return FinalStateError::unreadable_object;
  }
  if (name_space == 0 || !read_as_cstring(name_space, value.name_space)) {
    return FinalStateError::invalid_value;
  }
  value.has_config_group = false;
  result = std::move(value);
  return FinalStateError::ok;
}

FinalStateError project_registration_post_result_v23300(
    const std::uint32_t kind,
    const std::int32_t eax_result,
    RegistrationPostResult& result) noexcept {
  if (eax_result < 0) return FinalStateError::invalid_value;
  RegistrationPostResult value{};
  value.registration_kind = kind;
  switch (kind) {
    case GORE_AS_CAPTURE_REGISTRATION_OBJECT_TYPE_V1:
    case GORE_AS_CAPTURE_REGISTRATION_INTERFACE_V1:
    case GORE_AS_CAPTURE_REGISTRATION_ENUM_V1:
    case GORE_AS_CAPTURE_REGISTRATION_FUNCDEF_V1:
    case GORE_AS_CAPTURE_REGISTRATION_TYPEDEF_V1:
      value.semantic = RegistrationResultSemantic::engine_type_id;
      break;
    case GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1:
    case GORE_AS_CAPTURE_REGISTRATION_OBJECT_METHOD_V1:
    case GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1:
    case GORE_AS_CAPTURE_REGISTRATION_INTERFACE_METHOD_V1:
      value.semantic = RegistrationResultSemantic::engine_function_id;
      break;
    case GORE_AS_CAPTURE_REGISTRATION_GLOBAL_PROPERTY_V1:
      value.semantic = RegistrationResultSemantic::global_property_index;
      break;
    case GORE_AS_CAPTURE_REGISTRATION_OBJECT_PROPERTY_V1:
      value.semantic = RegistrationResultSemantic::object_property_index;
      break;
    case GORE_AS_CAPTURE_REGISTRATION_ENUM_VALUE_V1:
      value.semantic = RegistrationResultSemantic::enum_value_index;
      break;
    case GORE_AS_CAPTURE_REGISTRATION_STRING_FACTORY_V1:
    case GORE_AS_CAPTURE_REGISTRATION_DEFAULT_ARRAY_TYPE_V1:
      if (eax_result != 0) return FinalStateError::invalid_value;
      value.semantic = RegistrationResultSemantic::installed;
      value.installed = true;
      result = value;
      return FinalStateError::ok;
    default:
      return FinalStateError::invalid_argument;
  }
  value.value = static_cast<std::uint32_t>(eax_result);
  result = value;
  return FinalStateError::ok;
}

FinalStateError extract_object_type_final_state_v23300(
    const std::uintptr_t object,
    const std::uint32_t trace_type_id,
    const std::uint32_t public_byte_size,
    const std::uint32_t public_flags,
    const TypeIdResolver resolver,
    ObjectTypeFinalState& result) noexcept {
  if (object == 0 || resolver.resolve == nullptr) return FinalStateError::invalid_argument;
  try {
    ObjectTypeFinalState value{};
    value.type_id = trace_type_id;
    value.byte_size = public_byte_size;
    value.flags = public_flags;
    std::uintptr_t base = 0;
    std::uintptr_t shadow = 0;
    std::uint8_t accepts_value = 0;
    std::uint8_t accepts_reference = 0;
    std::uint8_t implicit_constructors = 0;
    std::uint8_t invalid_generated = 0;
    if (!read_value(object, layout::target_confirmed::object_type_alignment, value.alignment) ||
        value.alignment == 0 || value.alignment > kMaximumAlignment ||
        (value.alignment & (value.alignment - 1)) != 0 ||
        !read_value(object, layout::target_confirmed::object_type_base, base) ||
        !read_value(object, layout::target_confirmed::object_type_shadow, shadow) ||
        !read_value(
            object, layout::target_confirmed::object_type_accept_value_subtype, accepts_value) ||
        !read_value(
            object,
            layout::target_confirmed::object_type_accept_reference_subtype,
            accepts_reference) ||
        !read_value(
            object,
            layout::target_confirmed::object_type_implicit_constructors,
            implicit_constructors) ||
        !read_value(
            object,
            layout::target_confirmed::object_type_invalid_generated,
            invalid_generated)) {
      return FinalStateError::unreadable_object;
    }
    if (!bool_byte(accepts_value) || !bool_byte(accepts_reference) ||
        !bool_byte(implicit_constructors) || !bool_byte(invalid_generated)) {
      return FinalStateError::invalid_value;
    }
    if (!resolve_optional_type(
            base, resolver, value.has_base_type, value.base_type_id) ||
        !resolve_optional_type(
            shadow, resolver, value.has_shadow_type, value.shadow_type_id)) {
      return FinalStateError::unresolved_type_capability;
    }
    ArrayView interfaces{};
    ArrayView offsets{};
    if (!read_array(
            object,
            layout::target_confirmed::object_type_interfaces,
            sizeof(std::uintptr_t),
            interfaces) ||
        !read_array(
            object,
            layout::target_confirmed::object_type_interface_vft_offsets,
            sizeof(std::uint32_t),
            offsets)) {
      return FinalStateError::unreadable_object;
    }
    if (interfaces.length != offsets.length) return FinalStateError::invalid_value;
    value.interface_type_ids.reserve(interfaces.length);
    value.interface_vft_offsets.reserve(offsets.length);
    for (std::uint32_t index = 0; index < interfaces.length; ++index) {
      std::uintptr_t capability = 0;
      std::uint32_t type_id = 0;
      std::uint32_t vft_offset = 0;
      std::memcpy(
          &capability,
          reinterpret_cast<const void*>(
              interfaces.data + static_cast<std::size_t>(index) * sizeof(capability)),
          sizeof(capability));
      std::memcpy(
          &vft_offset,
          reinterpret_cast<const void*>(
              offsets.data + static_cast<std::size_t>(index) * sizeof(vft_offset)),
          sizeof(vft_offset));
      if (capability == 0 ||
          !resolver.resolve(resolver.context, capability, type_id)) {
        return FinalStateError::unresolved_type_capability;
      }
      value.interface_type_ids.push_back(type_id);
      value.interface_vft_offsets.push_back(vft_offset);
    }
    value.accepts_value_subtype = accepts_value != 0;
    value.accepts_reference_subtype = accepts_reference != 0;
    value.has_implicit_constructors = implicit_constructors != 0;
    value.is_invalid_generated_type = invalid_generated != 0;
    result = std::move(value);
    return FinalStateError::ok;
  } catch (...) {
    return FinalStateError::limit_exceeded;
  }
}

FinalStateError extract_object_property_final_state_v23300(
    const std::uintptr_t property,
    const std::uint32_t trace_property_id,
    ObjectPropertyFinalState& result) noexcept {
  if (property == 0) return FinalStateError::invalid_argument;
  ObjectPropertyFinalState value{};
  value.property_id = trace_property_id;
  std::uint8_t composite_indirect = 0;
  std::uint8_t is_private = 0;
  std::uint8_t is_protected = 0;
  std::uint8_t app_bind = 0;
  if (!read_value(property, layout::donor::object_property_byte_offset, value.byte_offset) ||
      !read_value(property, layout::donor::object_property_access_mask, value.access_mask) ||
      !read_value(
          property, layout::donor::object_property_composite_offset, value.composite_offset) ||
      !read_value(
          property,
          layout::donor::object_property_composite_indirect,
          composite_indirect) ||
      !read_value(property, layout::donor::object_property_private, is_private) ||
      !read_value(property, layout::donor::object_property_protected, is_protected) ||
      !read_value(property, layout::donor::object_property_app_bind, app_bind) ||
      !read_value(property, layout::donor::object_property_exposed_type, value.exposed_type)) {
    return FinalStateError::unreadable_object;
  }
  if (!bool_byte(composite_indirect) || !bool_byte(is_private) ||
      !bool_byte(is_protected) || !bool_byte(app_bind) || value.exposed_type > 0xff) {
    return FinalStateError::invalid_value;
  }
  value.is_composite_indirect = composite_indirect != 0;
  value.is_private = is_private != 0;
  value.is_protected = is_protected != 0;
  value.is_app_bind_property = app_bind != 0;
  result = value;
  return FinalStateError::ok;
}

FinalStateError extract_function_final_state_v23300(
    const std::uintptr_t function,
    const std::uint32_t trace_function_id,
    FunctionFinalState& result) noexcept {
  if (function == 0) return FinalStateError::invalid_argument;
  FunctionFinalState value{};
  value.function_id = trace_function_id;
  std::int8_t hidden_index = -1;
  std::int8_t output_index = -1;
  std::uintptr_t system_interface = 0;
  std::uint8_t first_param_metadata = 0;
  if (!read_value(function, layout::donor::script_function_traits, value.trait_bits) ||
      !read_value(function, layout::donor::script_function_exposed_type, value.exposed_type) ||
      !read_value(
          function, layout::donor::script_function_hidden_argument_index, hidden_index) ||
      !read_value(
          function,
          layout::donor::script_function_output_type_argument_index,
          output_index) ||
      !read_value(
          function, layout::donor::script_function_compile_out_type, value.compile_out_mode) ||
      !read_value(
          function, layout::donor::script_function_system_interface, system_interface)) {
    return FinalStateError::unreadable_object;
  }
  if (value.exposed_type > 0xff || value.compile_out_mode > 3 || hidden_index < -1 ||
      output_index < -1) {
    return FinalStateError::invalid_value;
  }
  if (hidden_index >= 0) {
    value.has_hidden_argument = true;
    value.hidden_argument_index = static_cast<std::uint8_t>(hidden_index);
    if (!read_as_cstring(
            function + layout::donor::script_function_hidden_argument_default,
            value.hidden_argument_default) ||
        value.hidden_argument_default.empty()) {
      return FinalStateError::invalid_value;
    }
  } else {
    std::string empty;
    if (!read_as_cstring(
            function + layout::donor::script_function_hidden_argument_default,
            empty) ||
        !empty.empty()) {
      return FinalStateError::invalid_value;
    }
  }
  if (output_index >= 0) {
    value.has_output_type_argument = true;
    value.output_type_argument_index = static_cast<std::uint8_t>(output_index);
  }
  if (system_interface != 0) {
    if (!read_value(
            system_interface,
            layout::donor::system_interface_first_param_metadata,
            first_param_metadata)) {
      return FinalStateError::unreadable_object;
    }
    if (first_param_metadata > 2) return FinalStateError::invalid_value;
    value.first_param_metadata = first_param_metadata;
  }
  result = std::move(value);
  return FinalStateError::ok;
}

FinalStateError extract_global_property_final_state_v23300(
    const std::uintptr_t property,
    const std::uint32_t trace_property_id,
    GlobalPropertyFinalState& result) noexcept {
  if (property == 0) return FinalStateError::invalid_argument;
  GlobalPropertyFinalState value{};
  value.property_id = trace_property_id;
  std::uint8_t pure = 0;
  if (!read_value(property, layout::donor::global_property_pure_constant, pure) ||
      !bool_byte(pure) ||
      (pure != 0 &&
       !read_value(
           property,
           layout::donor::global_property_storage,
           value.pure_constant_value))) {
    return FinalStateError::invalid_value;
  }
  value.is_pure_constant = pure != 0;
  result = value;
  return FinalStateError::ok;
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

template <typename Value>
void fixture_write(
    std::byte* const object,
    const std::size_t offset,
    const Value& value) noexcept {
  std::memcpy(object + offset, &value, sizeof(value));
}

bool fixture_type_id(
    void*,
    const std::uintptr_t capability,
    std::uint32_t& result) noexcept {
  if (capability < 100 || capability > 103) return false;
  result = static_cast<std::uint32_t>(capability + 900);
  return true;
}

}  // namespace

bool target_final_state_selftest_v23300() noexcept {
  constexpr std::array result_semantics{
      RegistrationResultSemantic::engine_function_id,
      RegistrationResultSemantic::global_property_index,
      RegistrationResultSemantic::engine_type_id,
      RegistrationResultSemantic::object_property_index,
      RegistrationResultSemantic::engine_function_id,
      RegistrationResultSemantic::engine_function_id,
      RegistrationResultSemantic::engine_type_id,
      RegistrationResultSemantic::engine_function_id,
      RegistrationResultSemantic::installed,
      RegistrationResultSemantic::installed,
      RegistrationResultSemantic::engine_type_id,
      RegistrationResultSemantic::enum_value_index,
      RegistrationResultSemantic::engine_type_id,
      RegistrationResultSemantic::engine_type_id,
  };
  for (std::size_t index = 0; index < result_semantics.size(); ++index) {
    RegistrationPostResult post_result{};
    const auto kind = static_cast<std::uint32_t>(index + 1);
    const auto eax_result =
        result_semantics[index] == RegistrationResultSemantic::installed ? 0 : 41;
    if (project_registration_post_result_v23300(kind, eax_result, post_result) !=
            FinalStateError::ok ||
        post_result.registration_kind != kind ||
        post_result.semantic != result_semantics[index] ||
        (post_result.semantic == RegistrationResultSemantic::installed
             ? !post_result.installed
             : post_result.value != 41)) {
      return false;
    }
  }
  RegistrationPostResult rejected_result{};
  if (project_registration_post_result_v23300(
          GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1,
          -1,
          rejected_result) != FinalStateError::invalid_value ||
      project_registration_post_result_v23300(
          GORE_AS_CAPTURE_REGISTRATION_STRING_FACTORY_V1,
          1,
          rejected_result) != FinalStateError::invalid_value ||
      project_registration_post_result_v23300(15, 0, rejected_result) !=
          FinalStateError::invalid_argument) {
    return false;
  }

  constexpr std::size_t kFixtureEngineBytes =
      layout::target_confirmed::target_engine_default_namespace +
      sizeof(std::uintptr_t);
  std::array<std::byte, kFixtureEngineBytes> engine{};
  std::array<std::byte, layout::donor::string_bytes> name_space{};
  constexpr char fixture_namespace[] = "Fixture::Bindings";
  fixture_write(
      name_space.data(),
      0,
      static_cast<std::uint32_t>(sizeof(fixture_namespace) - 1));
  fixture_write(
      name_space.data(),
      8,
      reinterpret_cast<std::uintptr_t>(fixture_namespace));
  fixture_write(
      engine.data(),
      layout::target_confirmed::target_engine_default_access_mask,
      0x55aa00ffu);
  fixture_write(
      engine.data(),
      layout::target_confirmed::target_engine_default_namespace,
      reinterpret_cast<std::uintptr_t>(name_space.data()));
  RegistrationContextFinalState context_result{};
  if (extract_registration_context_v23300(
          reinterpret_cast<std::uintptr_t>(engine.data()),
          context_result) != FinalStateError::ok ||
      context_result.name_space != fixture_namespace ||
      context_result.access_mask != 0x55aa00ffu ||
      context_result.has_config_group) {
    return false;
  }
  fixture_write(
      engine.data(),
      layout::target_confirmed::target_engine_default_namespace,
      std::uintptr_t{});
  if (extract_registration_context_v23300(
          reinterpret_cast<std::uintptr_t>(engine.data()),
          context_result) != FinalStateError::invalid_value) {
    return false;
  }

  std::array<std::byte, layout::target_confirmed::target_object_type_bytes> object{};
  const std::uint32_t alignment = 16;
  const std::uintptr_t base = 100;
  const std::uintptr_t shadow = 101;
  std::array<std::uintptr_t, 2> interfaces{102, 103};
  std::array<std::uint32_t, 2> vft_offsets{8, 24};
  fixture_write(object.data(), layout::target_confirmed::object_type_alignment, alignment);
  fixture_write(object.data(), layout::target_confirmed::object_type_base, base);
  fixture_write(object.data(), layout::target_confirmed::object_type_shadow, shadow);
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interfaces,
      reinterpret_cast<std::uintptr_t>(interfaces.data()));
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interfaces + sizeof(std::uintptr_t),
      static_cast<std::uint32_t>(interfaces.size()));
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interfaces + sizeof(std::uintptr_t) +
          sizeof(std::uint32_t),
      static_cast<std::uint32_t>(interfaces.size()));
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interface_vft_offsets,
      reinterpret_cast<std::uintptr_t>(vft_offsets.data()));
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interface_vft_offsets + sizeof(std::uintptr_t),
      static_cast<std::uint32_t>(vft_offsets.size()));
  fixture_write(
      object.data(),
      layout::target_confirmed::object_type_interface_vft_offsets + sizeof(std::uintptr_t) +
          sizeof(std::uint32_t),
      static_cast<std::uint32_t>(vft_offsets.size()));
  object[layout::target_confirmed::object_type_accept_value_subtype] = std::byte{1};
  object[layout::target_confirmed::object_type_accept_reference_subtype] = std::byte{0};
  object[layout::target_confirmed::object_type_implicit_constructors] = std::byte{1};
  object[layout::target_confirmed::object_type_invalid_generated] = std::byte{0};
  ObjectTypeFinalState object_result{};
  if (extract_object_type_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(object.data()),
          44,
          128,
          0x400,
          {nullptr, &fixture_type_id},
          object_result) != FinalStateError::ok ||
      object_result.type_id != 44 || object_result.byte_size != 128 ||
      object_result.alignment != 16 || object_result.flags != 0x400 ||
      !object_result.has_base_type || object_result.base_type_id != 1000 ||
      !object_result.has_shadow_type || object_result.shadow_type_id != 1001 ||
      object_result.interface_type_ids != std::vector<std::uint32_t>({1002, 1003}) ||
      object_result.interface_vft_offsets != std::vector<std::uint32_t>({8, 24}) ||
      !object_result.has_implicit_constructors ||
      !object_result.accepts_value_subtype || object_result.accepts_reference_subtype ||
      object_result.is_invalid_generated_type) {
    return false;
  }
  object[layout::target_confirmed::object_type_invalid_generated] = std::byte{2};
  if (extract_object_type_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(object.data()),
          44,
          128,
          0x400,
          {nullptr, &fixture_type_id},
          object_result) != FinalStateError::invalid_value) {
    return false;
  }

  std::array<std::byte, layout::donor::object_property_bytes> property{};
  fixture_write(property.data(), layout::donor::object_property_byte_offset, 48u);
  fixture_write(property.data(), layout::donor::object_property_access_mask, 0x55u);
  fixture_write(property.data(), layout::donor::object_property_composite_offset, 16u);
  property[layout::donor::object_property_composite_indirect] = std::byte{1};
  property[layout::donor::object_property_private] = std::byte{0};
  property[layout::donor::object_property_protected] = std::byte{1};
  property[layout::donor::object_property_app_bind] = std::byte{1};
  fixture_write(property.data(), layout::donor::object_property_exposed_type, 7u);
  ObjectPropertyFinalState property_result{};
  if (extract_object_property_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(property.data()),
          9,
          property_result) != FinalStateError::ok ||
      property_result.property_id != 9 || property_result.byte_offset != 48 ||
      property_result.access_mask != 0x55 || property_result.composite_offset != 16 ||
      !property_result.is_composite_indirect || property_result.is_private ||
      !property_result.is_protected || !property_result.is_app_bind_property ||
      property_result.exposed_type != 7) {
    return false;
  }
  property[layout::donor::object_property_private] = std::byte{3};
  if (extract_object_property_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(property.data()),
          9,
          property_result) != FinalStateError::invalid_value) {
    return false;
  }

  std::array<std::byte, layout::donor::script_function_bytes> function{};
  fixture_write(function.data(), layout::donor::script_function_traits, 0x1200u);
  fixture_write(function.data(), layout::donor::script_function_exposed_type, 5u);
  const std::int8_t hidden = 2;
  const std::int8_t output = 3;
  fixture_write(function.data(), layout::donor::script_function_hidden_argument_index, hidden);
  fixture_write(
      function.data(), layout::donor::script_function_output_type_argument_index, output);
  fixture_write(function.data(), layout::donor::script_function_compile_out_type, 3u);
  constexpr char hidden_default[] = "__WorldContext";
  fixture_write(
      function.data(),
      layout::donor::script_function_hidden_argument_default,
      static_cast<std::uint32_t>(sizeof(hidden_default) - 1));
  fixture_write(
      function.data(),
      layout::donor::script_function_hidden_argument_default + 8,
      reinterpret_cast<std::uintptr_t>(hidden_default));
  std::array<std::byte, layout::donor::system_interface_bytes> system{};
  fixture_write(
      system.data(), layout::donor::system_interface_first_param_metadata, 2u);
  fixture_write(
      function.data(),
      layout::donor::script_function_system_interface,
      reinterpret_cast<std::uintptr_t>(system.data()));
  FunctionFinalState function_result{};
  if (extract_function_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(function.data()),
          77,
          function_result) != FinalStateError::ok ||
      function_result.function_id != 77 || function_result.trait_bits != 0x1200 ||
      function_result.exposed_type != 5 || !function_result.has_hidden_argument ||
      function_result.hidden_argument_index != 2 ||
      function_result.hidden_argument_default != hidden_default ||
      !function_result.has_output_type_argument ||
      function_result.output_type_argument_index != 3 ||
      function_result.compile_out_mode != 3 || function_result.first_param_metadata != 2) {
    return false;
  }
  fixture_write(function.data(), layout::donor::script_function_compile_out_type, 4u);
  if (extract_function_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(function.data()),
          77,
          function_result) != FinalStateError::invalid_value) {
    return false;
  }

  std::array<std::byte, layout::donor::global_property_bytes> global{};
  fixture_write(global.data(), layout::donor::global_property_storage, 0x1122334455667788ull);
  global[layout::donor::global_property_pure_constant] = std::byte{1};
  GlobalPropertyFinalState global_result{};
  if (extract_global_property_final_state_v23300(
          reinterpret_cast<std::uintptr_t>(global.data()),
          12,
          global_result) != FinalStateError::ok ||
      global_result.property_id != 12 || !global_result.is_pure_constant ||
      global_result.pure_constant_value != 0x1122334455667788ull) {
    return false;
  }
  global[layout::donor::global_property_pure_constant] = std::byte{2};
  return extract_global_property_final_state_v23300(
             reinterpret_cast<std::uintptr_t>(global.data()),
             12,
             global_result) == FinalStateError::invalid_value;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
