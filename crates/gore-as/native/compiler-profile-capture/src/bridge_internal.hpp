#pragma once

#include <cstdint>

namespace gore_as_capture::v1::instrumentation {

// Read-only identity check used by the dormant instrumentation preflight. It appends nothing.
bool bridge_validate_live_session_v1(
    std::uint64_t session_id,
    std::uintptr_t primary_image) noexcept;

bool bridge_adopt_runtime_owner_v1(
    std::uint64_t session_id,
    std::uintptr_t primary_image) noexcept;

}  // namespace gore_as_capture::v1::instrumentation
