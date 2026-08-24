#pragma once

#include "gore_as_capture/format.hpp"

#include <cstdint>

namespace gore_as_capture::v1::instrumentation {

void live_capture_activate_control_v1(void* control) noexcept;
void live_capture_note_dispatch_failure_v1(
    std::uint32_t site,
    std::uint32_t phase) noexcept;
void live_capture_note_failure_detail_v1(std::uint32_t detail) noexcept;
void live_capture_note_container_header_v1(
    const std::array<std::uint64_t, 8>& header) noexcept;
bool live_capture_target_inputs_verified_v1() noexcept;
void live_capture_note_registration_result_v1(
    std::uint32_t site, std::int32_t result) noexcept;
void live_capture_note_registration_arguments_v1(
    const char* first, std::uint32_t first_bytes,
    const char* second, std::uint32_t second_bytes,
    std::uint64_t scalar0, std::uint64_t scalar1, std::uint64_t scalar2) noexcept;
void live_capture_note_type_layout_v1(
    std::uint32_t object_alignment,
    std::uint32_t operations_alignment,
    bool operations_available) noexcept;
void live_capture_note_reflected_type_v1(
    std::int32_t type_id,
    std::uint32_t operations_kind,
    std::uint32_t value_size,
    std::uint32_t value_alignment,
    bool operations_available) noexcept;
void live_capture_note_registry_counts_v1(
    const RegistryCounts& projected,
    const RegistryCounts& reflected) noexcept;
void live_capture_note_dispatch_timing_v1(std::uint64_t ticks) noexcept;
void live_capture_note_observer_stage_v1(std::uint32_t stage) noexcept;
void live_capture_note_outcome_v1(std::uint32_t outcome) noexcept;

}  // namespace gore_as_capture::v1::instrumentation
