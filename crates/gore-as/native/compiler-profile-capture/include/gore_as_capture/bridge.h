#pragma once

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#define GORE_AS_CAPTURE_CALL __cdecl
#if defined(GORE_AS_CAPTURE_BRIDGE_BUILD)
#define GORE_AS_CAPTURE_API __declspec(dllexport)
#else
#define GORE_AS_CAPTURE_API __declspec(dllimport)
#endif
#else
#define GORE_AS_CAPTURE_CALL
#define GORE_AS_CAPTURE_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum {
  GORE_AS_CAPTURE_BRIDGE_ABI_V1 = 1,
  GORE_AS_CAPTURE_BRIDGE_CONTRACT_BYTES_V1 = 112,
  GORE_AS_CAPTURE_ATTACH_REQUEST_BYTES_V1 = 96,
  GORE_AS_CAPTURE_HOOK_POINT_BYTES_V1 = 8,
  GORE_AS_CAPTURE_REGISTRY_COUNTS_BYTES_V1 = 32,
  GORE_AS_CAPTURE_BUILD_JIT_BYTES_V1 = 80,
  GORE_AS_CAPTURE_FRONTEND_BOUNDARY_BYTES_V1 = 116,
};

typedef enum gore_as_capture_bridge_error_v1 {
  GORE_AS_CAPTURE_BRIDGE_OK_V1 = 0,
  GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1 = 1,
  GORE_AS_CAPTURE_BRIDGE_ABI_MISMATCH_V1 = 2,
  GORE_AS_CAPTURE_BRIDGE_WRONG_TARGET_V1 = 3,
  GORE_AS_CAPTURE_BRIDGE_BUSY_V1 = 4,
  GORE_AS_CAPTURE_BRIDGE_WRONG_THREAD_V1 = 5,
  GORE_AS_CAPTURE_BRIDGE_INVALID_SESSION_V1 = 6,
  GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1 = 7,
  GORE_AS_CAPTURE_BRIDGE_OUTPUT_EXISTS_V1 = 8,
  GORE_AS_CAPTURE_BRIDGE_UNSAFE_OUTPUT_V1 = 9,
  GORE_AS_CAPTURE_BRIDGE_IO_ERROR_V1 = 10,
  GORE_AS_CAPTURE_BRIDGE_CRYPTO_ERROR_V1 = 11,
  GORE_AS_CAPTURE_BRIDGE_LIMIT_V1 = 12,
  GORE_AS_CAPTURE_BRIDGE_POINTER_OUTSIDE_IMAGE_V1 = 13,
  GORE_AS_CAPTURE_BRIDGE_RECOVERY_REQUIRED_V1 = 14,
} gore_as_capture_bridge_error_v1;

typedef struct gore_as_capture_bridge_contract_v1 {
  uint32_t struct_size;
  uint32_t abi_version;
  uint32_t hook_table_version;
  uint32_t hook_point_count;
  uint64_t hook_table_fingerprint;
  uint32_t steam_app_id;
  uint32_t reserved0;
  uint64_t steam_build_id;
  uint64_t executable_bytes;
  uint32_t pe_size_of_image;
  uint32_t codeview_age;
  uint8_t executable_sha256[32];
  uint8_t codeview_guid_rsds[16];
  uint32_t test_fixture_only;
  uint32_t reserved1;
} gore_as_capture_bridge_contract_v1;

typedef struct gore_as_capture_hook_point_v1 {
  uint32_t kind;
  uint32_t image_rva;
} gore_as_capture_hook_point_v1;

typedef struct gore_as_capture_attach_request_v1 {
  uint32_t struct_size;
  uint32_t abi_version;
  uint32_t hook_table_version;
  uint32_t reserved0;
  uint64_t hook_table_fingerprint;
  uint64_t observed_steam_build_id;
  uintptr_t primary_image_base;
  const wchar_t* executable_path;
  uint32_t executable_path_chars;
  uint32_t reserved1;
  const wchar_t* output_path;
  uint32_t output_path_chars;
  uint32_t reserved2;
  uint8_t capture_id[16];
  uint64_t reserved3;
} gore_as_capture_attach_request_v1;

typedef struct gore_as_capture_registry_counts_v1 {
  uint32_t types;
  uint32_t functions;
  uint32_t object_properties;
  uint32_t global_properties;
  uint32_t enum_values;
  uint32_t funcdefs;
  uint32_t typedefs;
  uint32_t total_registrations;
} gore_as_capture_registry_counts_v1;

typedef struct gore_as_capture_build_jit_v1 {
  uint32_t struct_size;
  uint32_t build_identifier;
  uint32_t shipping_cache_matches;
  uint32_t jit_info_present;
  uint32_t jit_guid_matches;
  uint32_t jit_database_cleared;
  uint32_t as_reference_debugging;
  uint32_t fork_opcode_table_201_212_present;
  uint32_t reference_debug_opcodes_emittable;
  uint32_t resolve_object_ptr_callback_registered;
  uint8_t precompiled_guid[16];
  uint8_t compiled_jit_guid[16];
  uint32_t get_build_identifier_rva;
  uint32_t get_static_jit_info_rva;
} gore_as_capture_build_jit_v1;

typedef struct gore_as_capture_frontend_boundary_v1 {
  uint32_t struct_size;
  uint32_t kind;
  uint32_t observation_rva;
  uint32_t module_count;
  int32_t result_code;
  uint8_t config_sha256[32];
  uint8_t input_sha256[32];
  uint8_t output_sha256[32];
} gore_as_capture_frontend_boundary_v1;

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_query_v1(gore_as_capture_bridge_contract_v1* contract_out);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_hook_point_v1(
    uint32_t index,
    gore_as_capture_hook_point_v1* point_out);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_attach_v1(
    const gore_as_capture_attach_request_v1* request,
    uint64_t* session_id_out);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_engine_property_v1(
    uint64_t session_id,
    uint32_t property_id,
    uint64_t value,
    uint32_t observation_rva);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_intern_primary_image_pointer_v1(
    uint64_t session_id,
    uintptr_t pointer,
    uint32_t* token_out);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_bind_begin_v1(
    uint64_t session_id,
    uint32_t callback_ordinal,
    int32_t bind_order,
    uint32_t callback_pointer_token,
    const gore_as_capture_registry_counts_v1* counts,
    const uint8_t registry_sha256[32]);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_bind_end_v1(
    uint64_t session_id,
    uint32_t callback_ordinal,
    int32_t bind_order,
    uint32_t callback_pointer_token,
    const gore_as_capture_registry_counts_v1* counts,
    const uint8_t registry_sha256[32]);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_registry_delta_json_v1(
    uint64_t session_id,
    const uint8_t* json,
    uint32_t json_bytes);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_post_bind_mutation_json_v1(
    uint64_t session_id,
    const uint8_t* json,
    uint32_t json_bytes);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_registry_support_json_v1(
    uint64_t session_id,
    const uint8_t* json,
    uint32_t json_bytes);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_final_post_bind_state_json_v1(
    uint64_t session_id,
    const uint8_t* json,
    uint32_t json_bytes);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_build_jit_v1(
    uint64_t session_id,
    const gore_as_capture_build_jit_v1* fact);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_frontend_config_json_v1(
    uint64_t session_id,
    uint32_t config_kind,
    const uint8_t* json,
    uint32_t json_bytes);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_frontend_boundary_v1(
    uint64_t session_id,
    const gore_as_capture_frontend_boundary_v1* boundary);

/// Seal, flush, close, and detach. Must run on the thread which attached the session.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_seal_and_detach_v1(uint64_t session_id);

/// Close and detach without sealing. The deliberately retained stream is diagnostic-only.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_abort_and_detach_v1(uint64_t session_id);

/// Returns OK only when no live session or instrumentation transaction remains. Success is a
/// terminal unload lease: every later attach or instrumentation preflight is refused.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_prepare_unload_v1(void);

#ifdef __cplusplus
}
#endif
