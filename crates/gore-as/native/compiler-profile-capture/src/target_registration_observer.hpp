#pragma once

#include "gore_as_capture/format.hpp"
#include "gore_as_capture/registration_hook_contract.hpp"
#include "target_capture_serializer.hpp"
#include "target_layout.hpp"
#include "target_type_usage.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

struct RawRegistrationArgument final {
  std::uint8_t semantic{};
  std::uint32_t text_bytes{};
  std::array<char, 1025> text{};
  std::uint64_t scalar{};
  std::uintptr_t pointer_capability{};
  std::array<std::byte, layout_v23300::donor::function_pointer_descriptor_bytes>
      opaque_descriptor{};
  std::uint32_t opaque_descriptor_bytes{};
};

struct RawRegistrationEntry final {
  std::uint32_t kind{};
  std::uintptr_t engine_capability{};
  std::uint32_t argument_count{};
  std::array<RawRegistrationArgument, registration::kMaximumArguments> arguments{};
};

enum class RegistrationObserverError : std::uint32_t {
  ok = 0,
  invalid_state,
  invalid_raw_frame,
  unreadable_target,
  abi_target_outside_image,
  unresolved_identity,
  result_rejected,
  type_operations_rejected,
  pointer_token_rejected,
  registry_count_drift,
  serialization_rejected,
  limit_exceeded,
  owner_declaration_missing,
  owner_correlation_missing,
  owner_record_missing,
  object_property_lookup_failed,
  object_property_correlation_failed,
  host_stub_derivation_failed,
  final_sequence_failed,
  type_reflection_failed,
  type_correlation_failed,
  type_operations_failed,
  object_type_layout_failed,
  object_type_alignment_invalid,
  global_property_type_operations_failed,
  global_property_type_operations_unavailable,
  global_property_zero_value_size,
};

struct PointerTokenResolver final {
  void* context{};
  bool (*resolve)(
      void* context,
      std::uintptr_t pointer_capability,
      std::uint32_t& pointer_token) noexcept{};
  bool (*resolve_object)(
      void* context,
      std::uintptr_t pointer_capability,
      std::uint32_t& pointer_token) noexcept{};
  bool (*resolve_storage)(
      void* context,
      std::uintptr_t pointer_capability,
      std::uint32_t& pointer_token) noexcept{};
};

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
// Test-only target facade. Production always uses the pinned public AS vtables, target-private
// arrays and FAngelscriptTypeUsage helper directly. The fixture facade cannot be exported.
struct RegistrationObserverTestTarget final {
  void* context{};
  bool (*type_by_id)(void*, std::int32_t, std::uintptr_t&) noexcept{};
  bool (*function_by_id)(void*, std::int32_t, std::uintptr_t&) noexcept{};
  bool (*type_by_declaration)(void*, const char*, std::uintptr_t&) noexcept{};
  bool (*global_property)(
      void*,
      std::uint32_t,
      std::int32_t&,
      std::uintptr_t&,
      std::uintptr_t&) noexcept{};
  bool (*object_property)(
      void*, std::uintptr_t, std::uint32_t, std::uintptr_t&) noexcept{};
  bool (*type_operations)(
      void*,
      std::int32_t,
      const char*,
      TargetTypeOperationsProjection&) noexcept{};
};
#endif

struct PendingRegistrationProjection final {
  RawRegistrationEntry raw{};
  RegistrationContextFinalState context{};
};

struct CompletedRegistrationProjection final {
  RegistrationEntryJsonProjection entry{};
  RegistrationResultJsonProjection result{};
  std::string delta_json;
};

// Stateful, exact-BuildID projection boundary. It is deliberately not a hook or injector: a
// future state-preserving wrapper supplies already extracted entry/return frames. Every completed
// registration is committed transactionally and retained for exact post-bind enumeration.
class TargetRegistrationObserver final {
 public:
  TargetRegistrationObserver(
      std::uintptr_t primary_image,
      std::uint32_t primary_image_bytes,
      std::uintptr_t engine_capability,
      PointerTokenResolver pointer_tokens
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
      ,
      RegistrationObserverTestTarget test_target = {}
#endif
      ) noexcept;

  RegistrationObserverError begin_observation(const RegistryCounts& baseline) noexcept;
  RegistrationObserverError prepare(
      const RawRegistrationEntry& raw,
      PendingRegistrationProjection& pending) const noexcept;
  bool is_core_intrinsic(
      const PendingRegistrationProjection& pending,
      std::int32_t eax_result) const noexcept;
  RegistrationObserverError complete(
      std::uint32_t bind_callback_ordinal,
      const PendingRegistrationProjection& pending,
      std::int32_t eax_result,
      CompletedRegistrationProjection& completed) noexcept;
  RegistrationObserverError finalize_registry(
      std::uint32_t bind_count,
      std::vector<std::vector<std::string>>& replacement_deltas,
      std::string& support_json) noexcept;
  RegistrationObserverError projected_counts(RegistryCounts& counts) const noexcept;
  RegistrationObserverError enumerate_post_bind_final_state(
      const RegistryCounts& final_counts,
      std::vector<std::string>& json_records) noexcept;

 private:
  enum class IdentityKind : std::uint32_t {
    object_type = 1,
    object_property,
    function,
    global_property,
  };
  struct TypeRecord final {
    std::uintptr_t capability{};
    std::uint32_t trace_id{};
    std::uint32_t engine_id{};
    std::uint32_t public_byte_size{};
    std::uint32_t public_alignment{1};
    std::uint32_t public_flags{};
    TypeOperationsJsonKind operations{TypeOperationsJsonKind::unavailable};
    std::string declaration;
    std::string name_space;
    bool refresh_operations{};
  };
  struct IdentityRecord final {
    IdentityKind kind{};
    std::uintptr_t capability{};
    std::uintptr_t owner_capability{};
    std::uintptr_t storage_capability{};
    std::uint32_t trace_id{};
    std::uint32_t engine_id{};
    std::uint32_t member_index{};
    std::uint32_t public_byte_size{};
    std::uint32_t public_flags{};
  };
  struct CompletedRecord final {
    std::uint32_t bind_ordinal{};
    RegistrationEntryJsonProjection entry;
    RegistrationResultJsonProjection result;
    std::int32_t global_storage_type_id{-1};
  };
  struct TypeProjectionRecord final {
    std::int32_t engine_type_id{};
    TargetTypeOperationsProjection projection;
  };

  RegistrationObserverError complete_in_place(
      std::uint32_t bind_callback_ordinal,
      const PendingRegistrationProjection& pending,
      std::int32_t eax_result,
      CompletedRegistrationProjection& completed) noexcept;
  RegistrationObserverError resolve_type_projection(
      std::int32_t engine_type_id,
      const std::string& declaration,
      TargetTypeOperationsProjection& projection,
      bool& available) const noexcept;
  RegistrationObserverError resolve_type_projection_cached(
      std::int32_t engine_type_id,
      const std::string& declaration,
      TargetTypeOperationsProjection& projection,
      bool& available) noexcept;
  RegistrationObserverError type_by_id(
      std::int32_t engine_type_id,
      std::uintptr_t& capability) const noexcept;
  RegistrationObserverError function_by_id(
      std::int32_t engine_function_id,
      std::uintptr_t& capability) const noexcept;
  RegistrationObserverError owner_by_declaration(
      const std::string& declaration,
      const std::string& name_space,
      std::uintptr_t& capability,
      std::uint32_t& trace_id,
      std::uint32_t& engine_id,
      TypeOperationsJsonKind& operations) const noexcept;
  RegistrationObserverError global_property(
      std::uint32_t index,
      std::int32_t& type_id,
      std::uintptr_t& storage,
      std::uintptr_t& property) const noexcept;
  RegistrationObserverError latest_global_property(
      std::uint32_t& index,
      std::int32_t& type_id,
      std::uintptr_t& storage,
      std::uintptr_t& property) const noexcept;
  RegistrationObserverError object_property(
      std::uintptr_t owner,
      std::uint32_t index,
      std::uintptr_t& property) const noexcept;
  RegistrationObserverError latest_object_property(
      std::uintptr_t owner,
      std::uint32_t& index,
      std::uintptr_t& property) const noexcept;
  RegistrationObserverError pointer_token(
      std::uintptr_t capability,
      std::uint32_t& token) const noexcept;
  RegistrationObserverError object_pointer_token(
      std::uintptr_t capability,
      std::uint32_t& token) const noexcept;
  RegistrationObserverError storage_pointer_token(
      std::uintptr_t capability,
      std::uint32_t& token) const noexcept;
  bool expected_counts_match(const RegistryCounts& final_counts) const noexcept;

  std::uintptr_t primary_image_{};
  std::uint32_t primary_image_bytes_{};
  std::uintptr_t engine_capability_{};
  PointerTokenResolver pointer_tokens_{};
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  RegistrationObserverTestTarget test_target_{};
#endif
  bool begun_{};
  bool finalized_{};
  RegistryCounts baseline_{};
  RegistryCounts expected_delta_{};
  TraceIdCorrelation correlation_{};
  HostStubCatalog host_stubs_{};
  FinalStateJsonSequence final_sequence_{};
  std::vector<TypeRecord> types_;
  std::vector<IdentityRecord> identities_;
  std::vector<CompletedRecord> completed_records_;
  std::vector<TypeProjectionRecord> type_projection_cache_;
};

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_registration_observer_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
