#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "gore_as_capture/instrumentation.h"

namespace gore_as_capture::v1::instrumentation {

enum class FinalStateError : std::uint32_t {
  ok = 0,
  invalid_argument,
  unreadable_object,
  invalid_value,
  limit_exceeded,
  unresolved_type_capability,
};

struct TypeIdResolver final {
  void* context{};
  bool (*resolve)(void* context, std::uintptr_t capability, std::uint32_t& type_id) noexcept{};
};

struct RegistrationContextFinalState final {
  std::string name_space;
  std::uint32_t access_mask{};
  // BuildID 24539464 maps Begin/End/RemoveConfigGroup to the same zero-return stub.
  // A non-empty config group is therefore not representable for this exact target.
  bool has_config_group{};
};

enum class RegistrationResultSemantic : std::uint32_t {
  engine_type_id = 1,
  engine_function_id,
  global_property_index,
  object_property_index,
  enum_value_index,
  installed,
};

struct RegistrationPostResult final {
  std::uint32_t registration_kind{};
  RegistrationResultSemantic semantic{};
  std::uint32_t value{};
  bool installed{};
};

struct ObjectTypeFinalState final {
  std::uint32_t type_id{};
  std::uint32_t byte_size{};
  std::uint32_t alignment{};
  std::uint32_t flags{};
  bool has_base_type{};
  std::uint32_t base_type_id{};
  bool has_shadow_type{};
  std::uint32_t shadow_type_id{};
  std::vector<std::uint32_t> interface_type_ids;
  std::vector<std::uint32_t> interface_vft_offsets;
  bool has_implicit_constructors{};
  bool accepts_value_subtype{};
  bool accepts_reference_subtype{};
  bool is_invalid_generated_type{};
};

struct ObjectPropertyFinalState final {
  std::uint32_t property_id{};
  std::uint32_t byte_offset{};
  std::uint32_t access_mask{};
  std::uint32_t composite_offset{};
  bool is_composite_indirect{};
  bool is_private{};
  bool is_protected{};
  bool is_app_bind_property{};
  std::uint32_t exposed_type{};
};

struct FunctionFinalState final {
  std::uint32_t function_id{};
  std::uint32_t trait_bits{};
  std::uint32_t exposed_type{};
  bool has_hidden_argument{};
  std::uint8_t hidden_argument_index{};
  std::string hidden_argument_default;
  bool has_output_type_argument{};
  std::uint8_t output_type_argument_index{};
  std::uint32_t compile_out_mode{};
  std::uint32_t first_param_metadata{};
};

struct GlobalPropertyFinalState final {
  std::uint32_t property_id{};
  bool is_pure_constant{};
  std::uint64_t pure_constant_value{};
};

FinalStateError extract_registration_context_v23300(
    std::uintptr_t engine_capability,
    RegistrationContextFinalState& result) noexcept;

FinalStateError project_registration_post_result_v23300(
    std::uint32_t registration_kind,
    std::int32_t eax_result,
    RegistrationPostResult& result) noexcept;

FinalStateError extract_object_type_final_state_v23300(
    std::uintptr_t object_type_capability,
    std::uint32_t trace_type_id,
    std::uint32_t public_byte_size,
    std::uint32_t public_flags,
    TypeIdResolver resolver,
    ObjectTypeFinalState& result) noexcept;

FinalStateError extract_object_property_final_state_v23300(
    std::uintptr_t property_capability,
    std::uint32_t trace_property_id,
    ObjectPropertyFinalState& result) noexcept;

FinalStateError extract_function_final_state_v23300(
    std::uintptr_t function_capability,
    std::uint32_t trace_function_id,
    FunctionFinalState& result) noexcept;

FinalStateError extract_global_property_final_state_v23300(
    std::uintptr_t property_capability,
    std::uint32_t trace_property_id,
    GlobalPropertyFinalState& result) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_final_state_selftest_v23300() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
