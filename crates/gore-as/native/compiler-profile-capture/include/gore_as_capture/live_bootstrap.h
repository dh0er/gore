#pragma once

#include <stdint.h>

#if defined(_WIN32)
#include <windows.h>
#endif

enum {
  GORE_AS_CAPTURE_LIVE_CONTROL_MAGIC_V1 = 0x314c4347u,
  GORE_AS_CAPTURE_LIVE_CONTROL_VERSION_V1 = 1,
  GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 = 1024,
  GORE_AS_CAPTURE_LIVE_STAGE_BUCKETS_V1 = 20,
  GORE_AS_CAPTURE_LIVE_PENDING_V1 = 0,
  GORE_AS_CAPTURE_LIVE_ENTERED_V1 = 1,
  GORE_AS_CAPTURE_LIVE_ATTACHED_V1 = 2,
  GORE_AS_CAPTURE_LIVE_INSTALLED_V1 = 3,
  GORE_AS_CAPTURE_LIVE_FAILED_V1 = 0x80000000u,
  GORE_AS_CAPTURE_LIVE_OUTCOME_PENDING_V1 = 0,
  GORE_AS_CAPTURE_LIVE_OUTCOME_SEALED_V1 = 1,
  GORE_AS_CAPTURE_LIVE_OUTCOME_ABORTED_V1 = 2,
  GORE_AS_CAPTURE_LIVE_OUTCOME_SEAL_FAILED_V1 = 3,
  GORE_AS_CAPTURE_LIVE_OUTCOME_ABORT_FAILED_V1 = 4,
};

typedef struct gore_as_capture_live_control_v1 {
  uint32_t struct_size;
  uint32_t magic;
  uint32_t version;
  volatile uint32_t status;
  uint32_t bridge_status;
  uint32_t image_validation_status;
  uint32_t patch_preflight_detail;
  uint32_t source_unwind_mask;
  uint32_t instrumentation_status;
  volatile uint32_t capture_outcome;
  volatile uint32_t failure_site;
  volatile uint32_t failure_phase;
  volatile uint32_t failure_detail;
  uint32_t capture_owner_thread;
  volatile uint32_t failure_thread;
  volatile uint32_t previous_registration_site;
  volatile int32_t previous_registration_result;
  volatile uint32_t last_registration_site;
  volatile int32_t last_registration_result;
  char last_registration_argument0[128];
  char last_registration_argument1[128];
  uint64_t last_registration_scalar0;
  uint64_t last_registration_scalar1;
  uint64_t last_registration_scalar2;
  uint32_t last_object_alignment;
  uint32_t last_operations_alignment;
  uint32_t last_operations_available;
  int32_t last_reflected_type_id;
  uint32_t last_type_operations_kind;
  uint32_t last_type_value_size;
  uint32_t projected_registry_counts[8];
  uint32_t reflected_registry_counts[8];
  uint64_t last_container_header[8];
  volatile uint64_t dispatch_ticks;
  volatile uint64_t dispatch_calls;
  volatile uint64_t registration_count;
  volatile uint32_t observer_stage;
  volatile uint64_t observer_stage_ticks[GORE_AS_CAPTURE_LIVE_STAGE_BUCKETS_V1];
  uint32_t executable_path_chars;
  uint32_t output_path_chars;
  uint64_t observed_steam_build_id;
  uint32_t target_inputs_verified;
  uint32_t reserved_target_inputs;
  uint8_t capture_id[16];
  wchar_t executable_path[GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1];
  wchar_t output_path[GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1];
} gore_as_capture_live_control_v1;

#if defined(_WIN32)
extern "C" __declspec(dllexport) VOID CALLBACK
gore_as_capture_live_bootstrap_v1(ULONG_PTR control_address);
#endif
