#pragma once

#include <stdint.h>

#include "gore_as_capture/bridge.h"

#ifdef __cplusplus
extern "C" {
#endif

enum {
  GORE_AS_CAPTURE_INSTRUMENTATION_ABI_V1 = 1,
  GORE_AS_CAPTURE_INSTRUMENTATION_CONTRACT_BYTES_V1 = 80,
  GORE_AS_CAPTURE_INSTRUMENTATION_SITE_CONTRACT_BYTES_V1 = 64,
  GORE_AS_CAPTURE_REGISTRATION_HOOK_SET_BYTES_V1 = 48,
  GORE_AS_CAPTURE_REGISTRATION_SITE_CONTRACT_BYTES_V1 = 104,
  GORE_AS_CAPTURE_INSTRUMENTATION_SELFTEST_BYTES_V1 = 40,
};

typedef enum gore_as_capture_instrumentation_transfer_v1 {
  GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1 = 1,
  GORE_AS_CAPTURE_TRANSFER_INLINE_JUMP_V1 = 2,
  GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1 = 3,
} gore_as_capture_instrumentation_transfer_v1;

typedef enum gore_as_capture_instrumentation_frame_v1 {
  GORE_AS_CAPTURE_FRAME_SET_ENGINE_PROPERTY_V1 = 1,
  GORE_AS_CAPTURE_FRAME_BIND_CALL_V1 = 2,
  GORE_AS_CAPTURE_FRAME_BIND_RETURN_V1 = 3,
  GORE_AS_CAPTURE_FRAME_BUILD_IDENTIFIER_V1 = 4,
  GORE_AS_CAPTURE_FRAME_STATIC_JIT_INFO_V1 = 5,
  GORE_AS_CAPTURE_FRAME_INITIAL_COMPILE_ENTER_V1 = 6,
  GORE_AS_CAPTURE_FRAME_PRECOMPILED_REQUEST_V1 = 7,
  GORE_AS_CAPTURE_FRAME_PREPROCESSOR_CONSTRUCTED_V1 = 8,
  GORE_AS_CAPTURE_FRAME_INITIAL_COMPILE_RETURN_V1 = 9,
} gore_as_capture_instrumentation_frame_v1;

enum {
  GORE_AS_CAPTURE_REGISTER_RAX_V1 = 1u << 0,
  GORE_AS_CAPTURE_REGISTER_RCX_V1 = 1u << 1,
  GORE_AS_CAPTURE_REGISTER_RDX_V1 = 1u << 2,
  GORE_AS_CAPTURE_REGISTER_R8_V1 = 1u << 3,
  GORE_AS_CAPTURE_REGISTER_RBX_V1 = 1u << 4,
  GORE_AS_CAPTURE_REGISTER_R12_V1 = 1u << 5,
  GORE_AS_CAPTURE_REGISTER_R15_V1 = 1u << 6,
  GORE_AS_CAPTURE_REGISTER_RDI_V1 = 1u << 7,
};

enum {
  GORE_AS_CAPTURE_SELFTEST_SITE_CONTRACT_V1 = 1u << 0,
  GORE_AS_CAPTURE_SELFTEST_RELOCATION_V1 = 1u << 1,
  GORE_AS_CAPTURE_SELFTEST_UNWIND_PLAN_V1 = 1u << 2,
  GORE_AS_CAPTURE_SELFTEST_TYPED_FRAMES_V1 = 1u << 3,
  GORE_AS_CAPTURE_SELFTEST_THREAD_WINDOW_V1 = 1u << 4,
  GORE_AS_CAPTURE_SELFTEST_PUBLIC_REGISTRY_SNAPSHOT_V1 = 1u << 5,
  GORE_AS_CAPTURE_SELFTEST_REGISTRATION_CONTRACT_V1 = 1u << 6,
  GORE_AS_CAPTURE_SELFTEST_REGISTRATION_TRANSACTION_V1 = 1u << 7,
  GORE_AS_CAPTURE_SELFTEST_REGISTRATION_UNWIND_V1 = 1u << 8,
  GORE_AS_CAPTURE_SELFTEST_REGISTRATION_ORDER_V1 = 1u << 9,
  GORE_AS_CAPTURE_SELFTEST_FINAL_STATE_EXTRACTOR_V1 = 1u << 10,
  GORE_AS_CAPTURE_SELFTEST_PRODUCTION_SHIMS_V1 = 1u << 11,
  GORE_AS_CAPTURE_SELFTEST_STATIC_RE_ALL_V1 = 0xfffu,
};

typedef enum gore_as_capture_registration_kind_v1 {
  GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1 = 1,
  GORE_AS_CAPTURE_REGISTRATION_GLOBAL_PROPERTY_V1 = 2,
  GORE_AS_CAPTURE_REGISTRATION_OBJECT_TYPE_V1 = 3,
  GORE_AS_CAPTURE_REGISTRATION_OBJECT_PROPERTY_V1 = 4,
  GORE_AS_CAPTURE_REGISTRATION_OBJECT_METHOD_V1 = 5,
  GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1 = 6,
  GORE_AS_CAPTURE_REGISTRATION_INTERFACE_V1 = 7,
  GORE_AS_CAPTURE_REGISTRATION_INTERFACE_METHOD_V1 = 8,
  GORE_AS_CAPTURE_REGISTRATION_STRING_FACTORY_V1 = 9,
  GORE_AS_CAPTURE_REGISTRATION_DEFAULT_ARRAY_TYPE_V1 = 10,
  GORE_AS_CAPTURE_REGISTRATION_ENUM_V1 = 11,
  GORE_AS_CAPTURE_REGISTRATION_ENUM_VALUE_V1 = 12,
  GORE_AS_CAPTURE_REGISTRATION_FUNCDEF_V1 = 13,
  GORE_AS_CAPTURE_REGISTRATION_TYPEDEF_V1 = 14,
} gore_as_capture_registration_kind_v1;

typedef enum gore_as_capture_registration_argument_source_v1 {
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_NONE_V1 = 0,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1 = 1,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1 = 2,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1 = 3,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1 = 4,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1 = 5,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1 = 6,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1 = 7,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1 = 8,
  GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1 = 9,
} gore_as_capture_registration_argument_source_v1;

typedef enum gore_as_capture_registration_argument_semantic_v1 {
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1 = 1,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1 = 2,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1 = 3,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1 = 4,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1 = 5,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1 = 6,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1 = 7,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1 = 8,
  GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1 = 9,
} gore_as_capture_registration_argument_semantic_v1;

enum {
  GORE_AS_CAPTURE_REGISTRATION_RETURN_EAX_I32_V1 = 1,
  GORE_AS_CAPTURE_REGISTRATION_CONTRACT_ENTRY_ORDER_V1 = 1u << 0,
  GORE_AS_CAPTURE_REGISTRATION_CONTRACT_RESULT_I32_V1 = 1u << 1,
  GORE_AS_CAPTURE_REGISTRATION_CONTRACT_AUXILIARY_TOKEN_V1 = 1u << 2,
  GORE_AS_CAPTURE_REGISTRATION_CONTRACT_CALLER_DESCRIPTOR_V1 = 1u << 3,
};

typedef enum gore_as_capture_instrumentation_error_v1 {
  GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 = 0,
  GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1 = 1,
  GORE_AS_CAPTURE_INSTRUMENTATION_ABI_MISMATCH_V1 = 2,
  GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1 = 3,
  GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1 = 4,
  GORE_AS_CAPTURE_INSTRUMENTATION_UNRESOLVED_SEMANTICS_V1 = 5,
  GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1 = 6,
  GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1 = 7,
  GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1 = 8,
  GORE_AS_CAPTURE_INSTRUMENTATION_ROLLBACK_FAILED_V1 = 9,
  GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1 = 10,
  GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1 = 11,
} gore_as_capture_instrumentation_error_v1;

typedef struct gore_as_capture_instrumentation_contract_v1 {
  uint32_t struct_size;
  uint32_t abi_version;
  uint64_t steam_build_id;
  uint32_t pe_size_of_image;
  uint32_t codeview_age;
  uint8_t codeview_guid_rsds[16];
  uint32_t hook_table_version;
  uint32_t hook_point_count;
  uint64_t hook_table_fingerprint;
  uint64_t prolog_table_fingerprint;
  uint32_t statically_extractable_hook_mask;
  uint32_t unresolved_hook_mask;
  uint32_t production_installable;
  uint32_t test_fixture_only;
} gore_as_capture_instrumentation_contract_v1;

/// Address-free static ABI/layout proof for one exact observation point. UINT32_MAX denotes an
/// inapplicable object offset. Register bits describe typed inputs/outputs of the wrapper; they
/// never authorize serializing a register value or process pointer.
typedef struct gore_as_capture_instrumentation_site_contract_v1 {
  uint32_t struct_size;
  uint32_t index;
  uint32_t hook_kind;
  uint32_t observation_rva;
  uint32_t patch_anchor_rva;
  uint32_t overwrite_bytes;
  uint32_t transfer_kind;
  uint32_t continuation_rva;
  uint32_t frame_kind;
  uint32_t register_read_mask;
  uint32_t manager_offset;
  uint32_t engine_offset;
  uint32_t result_offset;
  uint32_t record_stride;
  uint32_t direct_callee_rva;
  uint32_t reserved0;
} gore_as_capture_instrumentation_site_contract_v1;

/// Separate exact-build extension for the 14 central asCScriptEngine Register* entries. It is
/// intentionally not part of the original nine observation points or the capture wire ABI.
typedef struct gore_as_capture_registration_hook_set_v1 {
  uint32_t struct_size;
  uint32_t contract_version;
  uint32_t hook_count;
  uint32_t engine_vtable_rva;
  uint64_t table_fingerprint;
  uint64_t prolog_fingerprint;
  uint32_t statically_closed_hook_mask;
  uint32_t unresolved_hook_mask;
  uint32_t production_installable;
  uint32_t reserved0;
} gore_as_capture_registration_hook_set_v1;

/// Address-free entry ABI for one central registration function. Sources are relative to the
/// original caller RSP. Pointer-bearing fields are capabilities/tokens only, never wire values.
typedef struct gore_as_capture_registration_site_contract_v1 {
  uint32_t struct_size;
  uint32_t index;
  uint32_t registration_kind;
  uint32_t vtable_slot;
  uint32_t function_rva;
  uint32_t overwrite_bytes;
  uint32_t continuation_rva;
  uint32_t generated_unwind_prolog_bytes;
  uint32_t generated_unwind_operation_count;
  uint32_t argument_count;
  uint32_t return_source;
  uint32_t contract_flags;
  uint32_t source_unwind_info_rva;
  uint32_t source_prolog_bytes;
  uint8_t expected_prolog[24];
  uint8_t argument_sources[9];
  uint8_t argument_semantics[9];
  uint8_t reserved0[6];
} gore_as_capture_registration_site_contract_v1;

typedef struct gore_as_capture_instrumentation_selftest_v1 {
  uint32_t struct_size;
  uint32_t installed_all_nine;
  uint32_t restored_all_nine;
  uint32_t prolog_drift_refused_without_write;
  uint32_t injected_failure_rolled_back;
  uint32_t wrong_thread_refused;
  uint32_t unload_while_installed_refused;
  uint32_t record_order_exact;
  uint32_t record_order_drift_refused;
  uint32_t reserved0;
} gore_as_capture_instrumentation_selftest_v1;

/// Returns the exact target/prolog/extraction contract. It performs no target access or writes.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_v1(
    gore_as_capture_instrumentation_contract_v1* contract_out);

/// Returns one of exactly nine BuildID-pinned static site contracts.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_site_contract_v1(
    uint32_t index,
    gore_as_capture_instrumentation_site_contract_v1* contract_out);

/// Returns the exact-build central-registration extension. It performs no target access.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_registration_hook_set_v1(
    gore_as_capture_registration_hook_set_v1* contract_out);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_registration_site_v1(
    uint32_t index,
    gore_as_capture_registration_site_contract_v1* contract_out);

/// Read-only validation of the current primary image and all nine pinned instruction spans.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_validate_current_image_v1(uintptr_t primary_image_base);

/// Read-only exact-target patch-plan diagnostic used by the authorized early-bootstrap host.
/// detail_out receives the internal ProductionPatchError value; no executable byte is changed.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_diagnose_patch_preflight_v1(
    uintptr_t primary_image_base,
    uint32_t* detail_out);

/// Installs the complete exact-build transaction only after bridge/image/all-site preflight.
/// Any preflight refusal occurs before changing an executable byte; later failure aborts the
/// capture and exact uninstall remains required.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_install_v1(
    uint64_t session_id,
    uintptr_t primary_image_base);

GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_uninstall_v1(uint64_t session_id);

/// Returns OK only when no instrumentation patch transaction remains installed.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_prepare_unload_v1(void);

/// Direct adapter for the pinned SetEngineProperty entry: EDX is the original property id and R8
/// is the pointer-width value. The engine pointer is never accepted.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_observe_set_engine_property_v1(
    uint64_t session_id,
    uint32_t property_id_from_edx,
    uint64_t value_from_r8);

/// Runs only in the separately named test bridge. Production returns TEST_ONLY without mutation.
GORE_AS_CAPTURE_API uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_synthetic_selftest_v1(
    gore_as_capture_instrumentation_selftest_v1* result_out);

#ifdef __cplusplus
}
#endif
