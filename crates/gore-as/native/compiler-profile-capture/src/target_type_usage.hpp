#pragma once

#include "target_capture_serializer.hpp"

#include <cstddef>
#include <cstdint>

namespace gore_as_capture::v1::instrumentation {

// Exact BuildID-24539464 representation returned by
// FAngelscriptTypeUsage::FromTypeId (target RVA 0x474d8f0).  This intentionally does not model
// Unreal ownership: the target destructor at RVA 0x465c0d0 is the only code allowed to release a
// value returned by the target helper.
struct TargetTypeUsage final {
  std::uintptr_t subtypes{};
  std::int32_t subtype_count{};
  std::int32_t subtype_capacity{};
  std::uintptr_t type{};
  std::uintptr_t type_reference_controller{};
  std::uint8_t is_reference{};
  std::uint8_t is_const{};
  std::uint8_t reserved_22[6]{};
  std::uintptr_t script_class{};
};

static_assert(sizeof(TargetTypeUsage) == 0x30);
static_assert(offsetof(TargetTypeUsage, subtypes) == 0x00);
static_assert(offsetof(TargetTypeUsage, type) == 0x10);
static_assert(offsetof(TargetTypeUsage, is_reference) == 0x20);
static_assert(offsetof(TargetTypeUsage, is_const) == 0x21);
static_assert(offsetof(TargetTypeUsage, script_class) == 0x28);

struct TargetTypeOperationsProjection final {
  TypeOperationsJsonKind kind{TypeOperationsJsonKind::unavailable};
  FixedTypeOperationsProjection fixed{};
  std::uint32_t subtype_count{};
};

enum class TargetTypeUsageError : std::uint32_t {
  ok = 0,
  invalid_argument,
  unreadable_value,
  abi_target_outside_image,
  prolog_drift,
  invalid_operation_value,
  unresolved_type,
};

// Invokes only target-witnessed FAngelscriptType virtual slots. The value must be a live target
// TypeUsage. No raw target address enters the resulting pointer-neutral projection.
TargetTypeUsageError project_target_type_operations_v1(
    std::uintptr_t primary_image,
    std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    FixedTypeOperationsProjection& projection) noexcept;

TargetTypeUsageError project_target_type_operations_tree_v1(
    std::uintptr_t primary_image,
    std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    const char* declaration,
    std::size_t declaration_bytes,
    TargetTypeOperationsProjection& projection) noexcept;

// Resolves a public AngelScript type ID through the exact target helper, projects its fixed
// operations, and releases the temporary with the exact target destructor. Container heads are
// deliberately classified by the caller from the registered declaration after this projection.
TargetTypeUsageError resolve_target_type_operations_v1(
    std::uintptr_t primary_image,
    std::uint32_t image_bytes,
    std::int32_t engine_type_id,
    FixedTypeOperationsProjection& projection) noexcept;

// Resolves and recursively validates the complete TypeUsage tree. Container identity and arity
// are derived only from the registered declaration; every reachable subtype must have a valid,
// image-internal FAngelscriptType operation table. An unresolved root is reported distinctly so
// callers can encode the schema's explicit `unavailable` observation where that is permitted.
TargetTypeUsageError resolve_target_type_operations_projection_v1(
    std::uintptr_t primary_image,
    std::uint32_t image_bytes,
    std::int32_t engine_type_id,
    const char* declaration,
    std::size_t declaration_bytes,
    TargetTypeOperationsProjection& projection) noexcept;

TypeOperationsJsonKind classify_target_type_operations_v1(
    const char* declaration,
    std::size_t declaration_bytes) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_type_usage_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
