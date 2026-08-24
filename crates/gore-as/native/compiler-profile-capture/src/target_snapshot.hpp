#pragma once

#include "gore_as_capture/format.hpp"

#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

enum class SnapshotError : std::uint32_t {
  ok = 0,
  invalid_argument,
  unreadable_object,
  abi_target_outside_image,
  invalid_value,
  limit_exceeded,
  hash_failure,
};

struct PublicRegistrySnapshot final {
  RegistryCounts counts{};
  Digest canonical_sha256{};
};

// Process-local capabilities for object-type user data. The collector validates every public
// AngelScript vtable target against the pinned image and copies the type identity, but never
// serializes the capability. Callers must independently prove that the user-data object is a
// UClass before projecting it into the pointer-neutral profile.
struct NativeClassCapability final {
  std::string angelscript_type_name;
  std::string name_space;
  std::uintptr_t user_data{};
};

/// Enumerates only AngelScript 2.33.0's public, pointer-neutral registry projection. Every vtable
/// target must reside in the already pinned primary image. Auxiliary/storage/callable pointers
/// are deliberately excluded; they require the separately captured host-stub projection.
SnapshotError capture_public_registry_snapshot_v23300(
    std::uintptr_t primary_image,
    std::uint32_t primary_image_bytes,
    std::uintptr_t engine_capability,
    PublicRegistrySnapshot& snapshot_out) noexcept;

SnapshotError capture_native_class_capabilities_v23300(
    std::uintptr_t primary_image,
    std::uint32_t primary_image_bytes,
    std::uintptr_t engine_capability,
    std::vector<NativeClassCapability>& capabilities_out) noexcept;

/// Produces the canonical public-registry digest for a fresh AngelScript
/// engine before its first application/add-on registration.  This is used at
/// the first registration entry because AngelScript's public enumeration API
/// is not re-entrant while Register* holds its configuration lock.
SnapshotError empty_public_registry_snapshot_v23300(
    PublicRegistrySnapshot& snapshot_out) noexcept;

/// Advances an address-free, domain-separated witness for an observed public
/// registry mutation.  Intermediate bind boundaries use this witness instead
/// of repeatedly reflecting the complete registry.  The final boundary is
/// still replaced by a complete public-registry snapshot and therefore checks
/// the projected counts and all public state against the live engine.
SnapshotError advance_public_registry_witness_v1(
    const PublicRegistrySnapshot& previous,
    const RegistryCounts& projected_counts,
    std::string_view canonical_delta_json,
    PublicRegistrySnapshot& snapshot_out) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool public_registry_snapshot_selftest_v23300() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
