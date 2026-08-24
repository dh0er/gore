#pragma once

#include "target_final_state.hpp"

#include <array>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

enum class CaptureSerializationError : std::uint32_t {
  ok = 0,
  invalid_argument,
  invalid_utf8,
  duplicate_identity,
  unresolved_identity,
  descriptor_conflict,
  limit_exceeded,
  hash_failure,
};

enum class HostStubDescriptorKind : std::uint32_t { callable = 1, storage, object };

struct HostStubDescriptorProjection final {
  std::uint32_t stub_id{};
  HostStubDescriptorKind kind{};
  std::uint32_t pointer_token{};
  std::array<std::uint8_t, 32> semantic_sha256{};
  std::uint32_t byte_len{};
  std::uint32_t alignment{};
};

struct RegistrationEntryJsonProjection;

// Presence is explicit because pointer token zero is valid. Only the capability class required by
// a registration kind is accepted. Global-storage size/alignment are semantic type-usage results,
// never memory reads performed by the catalog.
struct RegistrationStubCapabilities final {
  bool has_callable{};
  std::uint32_t callable_pointer_token{};
  bool has_auxiliary_object{};
  std::uint32_t auxiliary_object_pointer_token{};
  bool has_storage{};
  std::uint32_t storage_pointer_token{};
  std::uint32_t storage_byte_len{};
  std::uint32_t storage_alignment{};
  bool has_factory_object{};
  std::uint32_t factory_object_pointer_token{};
};

class HostStubCatalog final {
 public:
  CaptureSerializationError derive_registration_stubs(
      RegistrationEntryJsonProjection& entry,
      const RegistrationStubCapabilities& capabilities) noexcept;
  CaptureSerializationError finalize(
      std::vector<HostStubDescriptorProjection>& projection) const noexcept;
  CaptureSerializationError update_storage_descriptor(
      std::uint32_t stub_id,
      std::uint32_t byte_len,
      std::uint32_t alignment) noexcept;

 private:
  struct Entry final {
    HostStubDescriptorKind kind{};
    std::uint32_t pointer_token{};
    std::uint32_t byte_len{};
    std::uint32_t alignment{};
    std::vector<std::string> witnesses;
  };
  CaptureSerializationError intern(
      HostStubDescriptorKind kind,
      std::uint32_t pointer_token,
      std::string_view semantic_witness,
      std::uint32_t byte_len,
      std::uint32_t alignment,
      std::uint32_t& stub_id) noexcept;
  CaptureSerializationError derive_registration_stubs_in_place(
      RegistrationEntryJsonProjection& entry,
      const RegistrationStubCapabilities& capabilities) noexcept;
  std::vector<Entry> entries_;
};

// Trace IDs are private capture identities. Engine IDs and member indices are retained only as
// correlation keys for PostBindResultV1; no engine-private ID is ever substituted for a trace ID.
class TraceIdCorrelation final {
 public:
  CaptureSerializationError claim_registration(
      std::uint32_t& ordinal,
      std::uint32_t& registration_id) noexcept;
  CaptureSerializationError register_type(
      std::uintptr_t type_capability,
      std::uint32_t engine_type_id,
      std::uint32_t& trace_type_id) noexcept;
  CaptureSerializationError register_function(
      std::uint32_t engine_function_id,
      std::uint32_t& trace_function_id) noexcept;
  CaptureSerializationError register_object_property(
      std::uintptr_t owner_type_capability,
      std::uint32_t property_index,
      std::uint32_t& trace_property_id,
      std::uint32_t& owner_engine_type_id) noexcept;
  CaptureSerializationError register_global_property(
      std::uint32_t global_property_index,
      std::uint32_t& trace_property_id) noexcept;
  CaptureSerializationError type_ids(
      std::uintptr_t type_capability,
      std::uint32_t& trace_type_id,
      std::uint32_t& engine_type_id) const noexcept;
  CaptureSerializationError trace_type_id_from_engine(
      std::uint32_t engine_type_id,
      std::uint32_t& trace_type_id) const noexcept;
  CaptureSerializationError trace_function_id_from_engine(
      std::uint32_t engine_function_id,
      std::uint32_t& trace_function_id) const noexcept;
  CaptureSerializationError trace_object_property_id(
      std::uint32_t owner_engine_type_id,
      std::uint32_t property_index,
      std::uint32_t& trace_property_id) const noexcept;
  CaptureSerializationError trace_global_property_id(
      std::uint32_t global_property_index,
      std::uint32_t& trace_property_id) const noexcept;

 private:
  struct TypeIdentity final {
    std::uintptr_t capability{};
    std::uint32_t trace_id{};
    std::uint32_t engine_id{};
  };
  struct PairIdentity final {
    std::uint32_t first{};
    std::uint32_t second{};
    std::uint32_t trace_id{};
  };
  struct ScalarIdentity final {
    std::uint32_t engine_id{};
    std::uint32_t trace_id{};
  };
  std::uint32_t registrations_{};
  std::uint32_t next_type_id_{};
  std::uint32_t next_function_id_{};
  std::uint32_t next_property_id_{};
  std::vector<TypeIdentity> types_;
  std::vector<ScalarIdentity> functions_;
  std::vector<PairIdentity> object_properties_;
  std::vector<ScalarIdentity> global_properties_;
};

enum class RegistrationEntryJsonKind : std::uint32_t {
  object_type = 1,
  interface,
  interface_method,
  object_property,
  object_method,
  object_behaviour,
  global_property,
  global_function,
  enumeration,
  enum_value,
  funcdef,
  type_alias,
  string_factory,
  default_array_type,
};

enum class TypeOperationsJsonKind : std::uint32_t {
  unavailable = 0,
  fixed,
  t_array,
  t_map,
  t_set,
  t_optional,
};

struct FixedTypeOperationsProjection final {
  bool can_create_property{};
  bool never_requires_gc{};
  bool requires_property{};
  bool can_be_template_subtype{};
  bool can_construct{};
  bool need_construct{};
  bool can_destruct{};
  bool need_destruct{};
  bool can_copy{};
  bool need_copy{};
  bool can_compare{};
  bool can_hash_value{};
  std::uint32_t value_size{};
  std::uint32_t value_alignment{};
  bool is_object_pointer{};
};

struct RegistrationEntryJsonProjection final {
  RegistrationEntryJsonKind kind{};
  std::uint32_t ordinal{};
  std::uint32_t registration_id{};
  RegistrationContextFinalState context;
  std::uint32_t trace_id{};
  std::uint32_t owner_trace_type_id{};
  std::string declaration;
  std::string name;
  std::string target_declaration;
  std::uint32_t byte_size{};
  std::uint32_t alignment{};
  std::uint32_t flags{};
  TypeOperationsJsonKind type_operations{TypeOperationsJsonKind::unavailable};
  FixedTypeOperationsProjection fixed_operations{};
  std::uint32_t byte_offset{};
  std::uint32_t composite_offset{};
  bool is_composite_indirect{};
  std::uint32_t accessor_type{};
  bool is_protected{};
  std::string call_convention;
  std::uint32_t callable_stub_id{};
  bool has_auxiliary_object_stub{};
  std::uint32_t auxiliary_object_stub_id{};
  std::string behaviour;
  bool has_template_validation_adapter{};
  std::string template_validation_adapter;
  std::int32_t enum_value{};
  std::uint32_t storage_stub_id{};
  std::uint32_t factory_object_stub_id{};
};

struct RegistrationResultJsonProjection final {
  RegistrationPostResult post_result;
  bool has_owner_engine_type_id{};
  std::uint32_t owner_engine_type_id{};
};

// Canonical 1:1 final-state order is the registration order filtered to object types, object
// properties, functions and global properties. A state can be emitted only once and only when it
// is the next expected trace identity; completion is false until every expected identity is seen.
class FinalStateJsonSequence final {
 public:
  CaptureSerializationError observe_registration(
      const RegistrationEntryJsonProjection& entry) noexcept;
  CaptureSerializationError begin_final_state() noexcept;
  CaptureSerializationError append(const ObjectTypeFinalState& state, std::string& json) noexcept;
  CaptureSerializationError append(
      const ObjectPropertyFinalState& state,
      std::string& json) noexcept;
  CaptureSerializationError append(const FunctionFinalState& state, std::string& json) noexcept;
  CaptureSerializationError append(
      const GlobalPropertyFinalState& state,
      std::string& json) noexcept;
  bool complete() const noexcept;

 private:
  enum class Kind : std::uint32_t { object_type = 1, object_property, function, global_property };
  struct Expected final {
    Kind kind{};
    std::uint32_t trace_id{};
  };
  CaptureSerializationError validate_next(Kind kind, std::uint32_t trace_id) noexcept;
  void commit_next() noexcept;
  std::uint32_t registrations_{};
  std::size_t next_{};
  bool registrations_closed_{};
  std::vector<Expected> expected_;
};

CaptureSerializationError serialize_registry_delta_json_v1(
    std::uint32_t bind_callback_ordinal,
    const RegistrationEntryJsonProjection& entry,
    const RegistrationResultJsonProjection& result,
    std::string& json) noexcept;

CaptureSerializationError serialize_registry_support_json_v1(
    const HostStubCatalog& catalog,
    std::string& json) noexcept;

CaptureSerializationError serialize_final_state_json_v1(
    std::uint32_t state_ordinal,
    const ObjectTypeFinalState& state,
    std::string& json) noexcept;
CaptureSerializationError serialize_final_state_json_v1(
    std::uint32_t state_ordinal,
    const ObjectPropertyFinalState& state,
    std::string& json) noexcept;
CaptureSerializationError serialize_final_state_json_v1(
    std::uint32_t state_ordinal,
    const FunctionFinalState& state,
    std::string& json) noexcept;
CaptureSerializationError serialize_final_state_json_v1(
    std::uint32_t state_ordinal,
    const GlobalPropertyFinalState& state,
    std::string& json) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_capture_serializer_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
