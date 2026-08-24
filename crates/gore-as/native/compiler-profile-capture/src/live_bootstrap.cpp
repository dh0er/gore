#define GORE_AS_CAPTURE_BRIDGE_BUILD

#include "gore_as_capture/live_bootstrap.h"

#include "gore_as_capture/bridge.h"
#include "gore_as_capture/instrumentation.h"
#include "live_bootstrap_internal.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>

namespace {

std::atomic<gore_as_capture_live_control_v1*> g_live_control{};
thread_local std::uint32_t g_observer_stage{};
thread_local LARGE_INTEGER g_observer_stage_started{};

std::size_t observer_stage_bucket(const std::uint32_t stage) noexcept {
  constexpr std::array<std::uint32_t, GORE_AS_CAPTURE_LIVE_STAGE_BUCKETS_V1> stages{
      0x100u, 0x101u, 0x102u, 0x103u, 0x200u, 0x300u,
      0x301u, 0x302u, 0x303u, 0x304u, 0x305u, 0x306u,
      0x307u, 0x308u, 0x400u, 0x401u, 0x402u, 0x403u,
      0x500u, 0x501u};
  const auto found = std::find(stages.begin(), stages.end(), stage);
  return found == stages.end() ? stages.size()
                               : static_cast<std::size_t>(found - stages.begin());
}

void publish_status(
    gore_as_capture_live_control_v1& control,
    const std::uint32_t status) noexcept {
  (void)InterlockedExchange(
      reinterpret_cast<volatile LONG*>(&control.status),
      static_cast<LONG>(status));
}

bool valid_control(const gore_as_capture_live_control_v1& control) noexcept {
  return control.struct_size == sizeof(control) &&
         control.magic == GORE_AS_CAPTURE_LIVE_CONTROL_MAGIC_V1 &&
         control.version == GORE_AS_CAPTURE_LIVE_CONTROL_VERSION_V1 &&
         control.status == GORE_AS_CAPTURE_LIVE_PENDING_V1 &&
         control.observed_steam_build_id != 0 &&
         control.target_inputs_verified == 1 &&
         control.executable_path_chars != 0 &&
         control.executable_path_chars < GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 &&
         control.output_path_chars != 0 &&
         control.output_path_chars < GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 &&
         control.executable_path[control.executable_path_chars] == L'\0' &&
         control.output_path[control.output_path_chars] == L'\0' &&
         std::any_of(
             std::begin(control.capture_id),
             std::end(control.capture_id),
             [](const std::uint8_t value) { return value != 0; });
}

}  // namespace

bool gore_as_capture::v1::instrumentation::
live_capture_target_inputs_verified_v1() noexcept {
  const auto* const control = g_live_control.load(std::memory_order_acquire);
  return control != nullptr && control->target_inputs_verified == 1;
}

void gore_as_capture::v1::instrumentation::live_capture_activate_control_v1(
    void* const control) noexcept {
  g_live_control.store(
      static_cast<gore_as_capture_live_control_v1*>(control),
      std::memory_order_release);
}

void gore_as_capture::v1::instrumentation::live_capture_note_dispatch_failure_v1(
    const std::uint32_t site,
    const std::uint32_t phase) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  (void)InterlockedCompareExchange(
      reinterpret_cast<volatile LONG*>(&control->failure_site),
      static_cast<LONG>(site),
      static_cast<LONG>(std::numeric_limits<std::uint32_t>::max()));
  (void)InterlockedCompareExchange(
      reinterpret_cast<volatile LONG*>(&control->failure_phase),
      static_cast<LONG>(phase),
      static_cast<LONG>(std::numeric_limits<std::uint32_t>::max()));
  (void)InterlockedCompareExchange(
      reinterpret_cast<volatile LONG*>(&control->failure_thread),
      static_cast<LONG>(GetCurrentThreadId()),
      0);
}

void gore_as_capture::v1::instrumentation::live_capture_note_outcome_v1(
    const std::uint32_t outcome) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  (void)InterlockedCompareExchange(
      reinterpret_cast<volatile LONG*>(&control->capture_outcome),
      static_cast<LONG>(outcome),
      GORE_AS_CAPTURE_LIVE_OUTCOME_PENDING_V1);
}

void gore_as_capture::v1::instrumentation::live_capture_note_failure_detail_v1(
    const std::uint32_t detail) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  (void)InterlockedCompareExchange(
      reinterpret_cast<volatile LONG*>(&control->failure_detail),
      static_cast<LONG>(detail),
      static_cast<LONG>(std::numeric_limits<std::uint32_t>::max()));
}

void gore_as_capture::v1::instrumentation::live_capture_note_container_header_v1(
    const std::array<std::uint64_t, 8>& header) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  std::copy(header.begin(), header.end(), std::begin(control->last_container_header));
}

void gore_as_capture::v1::instrumentation::live_capture_note_registration_result_v1(
    const std::uint32_t site,
    const std::int32_t result) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  control->previous_registration_site = control->last_registration_site;
  control->previous_registration_result = control->last_registration_result;
  control->last_registration_site = site;
  control->last_registration_result = result;
  (void)InterlockedIncrement64(
      reinterpret_cast<volatile LONG64*>(&control->registration_count));
}

void gore_as_capture::v1::instrumentation::live_capture_note_dispatch_timing_v1(
    const std::uint64_t ticks) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr || ticks > static_cast<std::uint64_t>(MAXLONGLONG)) return;
  (void)InterlockedAdd64(
      reinterpret_cast<volatile LONG64*>(&control->dispatch_ticks),
      static_cast<LONG64>(ticks));
  (void)InterlockedIncrement64(
      reinterpret_cast<volatile LONG64*>(&control->dispatch_calls));
}

void gore_as_capture::v1::instrumentation::live_capture_note_registry_counts_v1(
    const RegistryCounts& projected,
    const RegistryCounts& reflected) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  const std::array projected_values{
      projected.types, projected.functions, projected.object_properties,
      projected.global_properties, projected.enum_values, projected.funcdefs,
      projected.typedefs, projected.total_registrations};
  const std::array reflected_values{
      reflected.types, reflected.functions, reflected.object_properties,
      reflected.global_properties, reflected.enum_values, reflected.funcdefs,
      reflected.typedefs, reflected.total_registrations};
  std::copy(
      projected_values.begin(), projected_values.end(),
      std::begin(control->projected_registry_counts));
  std::copy(
      reflected_values.begin(), reflected_values.end(),
      std::begin(control->reflected_registry_counts));
}

void gore_as_capture::v1::instrumentation::live_capture_note_observer_stage_v1(
    const std::uint32_t stage) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  LARGE_INTEGER now{};
  if (QueryPerformanceCounter(&now) != FALSE && g_observer_stage != 0 &&
      g_observer_stage_started.QuadPart != 0 &&
      now.QuadPart >= g_observer_stage_started.QuadPart) {
    const auto bucket = observer_stage_bucket(g_observer_stage);
    const auto elapsed = static_cast<std::uint64_t>(
        now.QuadPart - g_observer_stage_started.QuadPart);
    if (bucket < GORE_AS_CAPTURE_LIVE_STAGE_BUCKETS_V1 &&
        elapsed <= static_cast<std::uint64_t>(MAXLONGLONG)) {
      (void)InterlockedAdd64(
          reinterpret_cast<volatile LONG64*>(
              &control->observer_stage_ticks[bucket]),
          static_cast<LONG64>(elapsed));
    }
  }
  g_observer_stage = stage;
  g_observer_stage_started = now;
  (void)InterlockedExchange(
      reinterpret_cast<volatile LONG*>(&control->observer_stage),
      static_cast<LONG>(stage));
}

void gore_as_capture::v1::instrumentation::live_capture_note_registration_arguments_v1(
    const char* const first,
    const std::uint32_t first_bytes,
    const char* const second,
    const std::uint32_t second_bytes,
    const std::uint64_t scalar0,
    const std::uint64_t scalar1,
    const std::uint64_t scalar2) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  const auto copy = [](char (&output)[128], const char* input,
                       const std::uint32_t input_bytes) noexcept {
    const auto bytes = input == nullptr
                           ? 0u
                           : std::min<std::uint32_t>(input_bytes, sizeof(output) - 1);
    if (bytes != 0) std::memcpy(output, input, bytes);
    output[bytes] = '\0';
  };
  copy(control->last_registration_argument0, first, first_bytes);
  copy(control->last_registration_argument1, second, second_bytes);
  control->last_registration_scalar0 = scalar0;
  control->last_registration_scalar1 = scalar1;
  control->last_registration_scalar2 = scalar2;
}

void gore_as_capture::v1::instrumentation::live_capture_note_type_layout_v1(
    const std::uint32_t object_alignment,
    const std::uint32_t operations_alignment,
    const bool operations_available) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  control->last_object_alignment = object_alignment;
  control->last_operations_alignment = operations_alignment;
  control->last_operations_available = operations_available ? 1u : 0u;
}

void gore_as_capture::v1::instrumentation::live_capture_note_reflected_type_v1(
    const std::int32_t type_id,
    const std::uint32_t operations_kind,
    const std::uint32_t value_size,
    const std::uint32_t value_alignment,
    const bool operations_available) noexcept {
  auto* const control = g_live_control.load(std::memory_order_acquire);
  if (control == nullptr) return;
  control->last_reflected_type_id = type_id;
  control->last_type_operations_kind = operations_kind;
  control->last_type_value_size = value_size;
  control->last_operations_alignment = value_alignment;
  control->last_operations_available = operations_available ? 1u : 0u;
}

extern "C" __declspec(dllexport) VOID CALLBACK
gore_as_capture_live_bootstrap_v1(const ULONG_PTR control_address) {
  auto* const control =
      reinterpret_cast<gore_as_capture_live_control_v1*>(control_address);
  if (control == nullptr || !valid_control(*control)) return;
  control->failure_site = std::numeric_limits<std::uint32_t>::max();
  control->failure_phase = std::numeric_limits<std::uint32_t>::max();
  control->failure_detail = std::numeric_limits<std::uint32_t>::max();
  control->previous_registration_site = std::numeric_limits<std::uint32_t>::max();
  control->previous_registration_result = 0;
  control->last_registration_site = std::numeric_limits<std::uint32_t>::max();
  control->last_registration_result = 0;
  control->last_registration_argument0[0] = '\0';
  control->last_registration_argument1[0] = '\0';
  control->last_registration_scalar0 = 0;
  control->last_registration_scalar1 = 0;
  control->last_registration_scalar2 = 0;
  control->last_object_alignment = 0;
  control->last_operations_alignment = 0;
  control->last_operations_available = 0;
  control->last_reflected_type_id = -1;
  control->last_type_operations_kind = 0;
  control->last_type_value_size = 0;
  std::fill(
      std::begin(control->projected_registry_counts),
      std::end(control->projected_registry_counts), 0);
  std::fill(
      std::begin(control->reflected_registry_counts),
      std::end(control->reflected_registry_counts), 0);
  std::fill(
      std::begin(control->last_container_header),
      std::end(control->last_container_header), 0);
  control->dispatch_ticks = 0;
  control->dispatch_calls = 0;
  control->registration_count = 0;
  control->observer_stage = 0;
  std::fill(
      std::begin(control->observer_stage_ticks),
      std::end(control->observer_stage_ticks),
      0);
  control->capture_owner_thread = GetCurrentThreadId();
  publish_status(*control, GORE_AS_CAPTURE_LIVE_ENTERED_V1);

  gore_as_capture_bridge_contract_v1 bridge_contract{};
  gore_as_capture_instrumentation_contract_v1 instrumentation_contract{};
  gore_as_capture_registration_hook_set_v1 registration_contract{};
  if (gore_as_capture_bridge_query_v1(&bridge_contract) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      gore_as_capture_instrumentation_query_v1(&instrumentation_contract) !=
          GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 ||
      gore_as_capture_instrumentation_query_registration_hook_set_v1(
          &registration_contract) != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 ||
      bridge_contract.test_fixture_only != 0 ||
      bridge_contract.steam_build_id != control->observed_steam_build_id ||
      instrumentation_contract.test_fixture_only != 0 ||
      instrumentation_contract.production_installable != 1 ||
      instrumentation_contract.steam_build_id != control->observed_steam_build_id ||
      registration_contract.production_installable != 1) {
    control->bridge_status = GORE_AS_CAPTURE_BRIDGE_WRONG_TARGET_V1;
    control->instrumentation_status = GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
    publish_status(
        *control, static_cast<std::uint32_t>(GORE_AS_CAPTURE_LIVE_FAILED_V1));
    return;
  }

  gore_as_capture_attach_request_v1 request{};
  request.struct_size = sizeof(request);
  request.abi_version = bridge_contract.abi_version;
  request.hook_table_version = bridge_contract.hook_table_version;
  request.hook_table_fingerprint = bridge_contract.hook_table_fingerprint;
  request.observed_steam_build_id = control->observed_steam_build_id;
  request.primary_image_base = reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr));
  request.executable_path = control->executable_path;
  request.executable_path_chars = control->executable_path_chars;
  request.output_path = control->output_path;
  request.output_path_chars = control->output_path_chars;
  std::copy(
      std::begin(control->capture_id),
      std::end(control->capture_id),
      std::begin(request.capture_id));

  // Validate the live image before opening the capture session.  This keeps a
  // bridge target refusal diagnosable: a zero value no longer ambiguously
  // means that image validation was never attempted.
  control->image_validation_status =
      gore_as_capture_instrumentation_validate_current_image_v1(
          request.primary_image_base);
  if (control->image_validation_status != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) {
    publish_status(
        *control, static_cast<std::uint32_t>(GORE_AS_CAPTURE_LIVE_FAILED_V1));
    return;
  }

  std::uint64_t session_id = 0;
  control->bridge_status = gore_as_capture_bridge_attach_v1(&request, &session_id);
  if (control->bridge_status != GORE_AS_CAPTURE_BRIDGE_OK_V1 || session_id == 0) {
    publish_status(
        *control, static_cast<std::uint32_t>(GORE_AS_CAPTURE_LIVE_FAILED_V1));
    return;
  }
  publish_status(*control, GORE_AS_CAPTURE_LIVE_ATTACHED_V1);
  const auto patch_diagnostic =
      gore_as_capture_instrumentation_diagnose_patch_preflight_v1(
          request.primary_image_base, &control->patch_preflight_detail);
  for (std::uint32_t index = 0; index < instrumentation_contract.hook_point_count; ++index) {
    gore_as_capture_instrumentation_site_contract_v1 site{};
    if (gore_as_capture_instrumentation_query_site_contract_v1(index, &site) !=
            GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 ||
        site.transfer_kind == GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1) {
      continue;
    }
    DWORD64 image_base = 0;
    if (RtlLookupFunctionEntry(
            request.primary_image_base + site.patch_anchor_rva,
            &image_base,
            nullptr) != nullptr &&
        image_base == request.primary_image_base) {
      control->source_unwind_mask |= 1u << index;
    }
  }
  if (patch_diagnostic != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) {
    control->instrumentation_status = patch_diagnostic;
    (void)gore_as_capture_bridge_abort_and_detach_v1(session_id);
    publish_status(
        *control, static_cast<std::uint32_t>(GORE_AS_CAPTURE_LIVE_FAILED_V1));
    return;
  }
  gore_as_capture::v1::instrumentation::live_capture_activate_control_v1(control);
  control->instrumentation_status = gore_as_capture_instrumentation_install_v1(
      session_id, request.primary_image_base);
  if (control->instrumentation_status != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) {
    (void)gore_as_capture_bridge_abort_and_detach_v1(session_id);
    publish_status(
        *control, static_cast<std::uint32_t>(GORE_AS_CAPTURE_LIVE_FAILED_V1));
    return;
  }
  publish_status(*control, GORE_AS_CAPTURE_LIVE_INSTALLED_V1);
}
