#define GORE_AS_CAPTURE_BRIDGE_BUILD
#include "gore_as_capture/bridge.h"
#include "gore_as_capture/instrumentation.h"

#include "gore_as_capture/hook_table.hpp"
#include "gore_as_capture/session.hpp"
#include "bridge_internal.hpp"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <limits>
#include <memory>
#include <mutex>
#include <new>
#include <span>
#include <string>
#include <string_view>

namespace {

using gore_as_capture::v1::BuildJitFact;
using gore_as_capture::v1::CaptureError;
using gore_as_capture::v1::CaptureSession;
using gore_as_capture::v1::Digest;
using gore_as_capture::v1::FrontendBoundary;
using gore_as_capture::v1::FrontendBoundaryKind;
using gore_as_capture::v1::GuidBytes;
using gore_as_capture::v1::RegistryCounts;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
constexpr std::uint64_t kExpectedObservedBuildId = 0xf17e'2487'8692'0001ull;
constexpr std::uint32_t kTestFixtureOnly = 1;
#else
constexpr std::uint64_t kExpectedObservedBuildId = gore_as_capture::v1::kSteamBuildId;
constexpr std::uint32_t kTestFixtureOnly = 0;
#endif

constexpr std::size_t kMaxPathChars = 32767;
constexpr std::uint64_t kSessionSeed = 0x474f'5245'4153'4301ull;

static_assert(sizeof(gore_as_capture_bridge_contract_v1) ==
              GORE_AS_CAPTURE_BRIDGE_CONTRACT_BYTES_V1);
static_assert(sizeof(gore_as_capture_attach_request_v1) ==
              GORE_AS_CAPTURE_ATTACH_REQUEST_BYTES_V1);
static_assert(sizeof(gore_as_capture_hook_point_v1) == GORE_AS_CAPTURE_HOOK_POINT_BYTES_V1);
static_assert(sizeof(gore_as_capture_registry_counts_v1) ==
              GORE_AS_CAPTURE_REGISTRY_COUNTS_BYTES_V1);
static_assert(sizeof(gore_as_capture_build_jit_v1) == GORE_AS_CAPTURE_BUILD_JIT_BYTES_V1);
static_assert(sizeof(gore_as_capture_frontend_boundary_v1) ==
              GORE_AS_CAPTURE_FRONTEND_BOUNDARY_BYTES_V1);

struct BridgeState final {
  std::mutex mutex;
  std::unique_ptr<CaptureSession> session;
  std::uint64_t session_id{};
  DWORD owner_thread{};
  std::uintptr_t primary_image{};
  std::uint64_t next_generation{1};
  bool unload_preparing{};
  bool unload_prepared{};
};

BridgeState& bridge_state() {
  static BridgeState state;
  return state;
}

std::uint32_t bridge_error(const CaptureError error) noexcept {
  switch (error) {
    case CaptureError::ok:
      return GORE_AS_CAPTURE_BRIDGE_OK_V1;
    case CaptureError::invalid_argument:
      return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
    case CaptureError::wrong_target:
      return GORE_AS_CAPTURE_BRIDGE_WRONG_TARGET_V1;
    case CaptureError::unsafe_output_path:
      return GORE_AS_CAPTURE_BRIDGE_UNSAFE_OUTPUT_V1;
    case CaptureError::output_exists:
      return GORE_AS_CAPTURE_BRIDGE_OUTPUT_EXISTS_V1;
    case CaptureError::io_error:
      return GORE_AS_CAPTURE_BRIDGE_IO_ERROR_V1;
    case CaptureError::crypto_error:
      return GORE_AS_CAPTURE_BRIDGE_CRYPTO_ERROR_V1;
    case CaptureError::size_limit:
    case CaptureError::record_limit:
      return GORE_AS_CAPTURE_BRIDGE_LIMIT_V1;
    case CaptureError::invalid_state:
    case CaptureError::duplicate_or_late_record:
      return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
    case CaptureError::pointer_outside_primary_image:
      return GORE_AS_CAPTURE_BRIDGE_POINTER_OUTSIDE_IMAGE_V1;
    case CaptureError::output_recovery_required:
      return GORE_AS_CAPTURE_BRIDGE_RECOVERY_REQUIRED_V1;
  }
  return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
}

std::span<const std::byte> json_span(const std::uint8_t* bytes, const std::uint32_t size) noexcept {
  return {reinterpret_cast<const std::byte*>(bytes), size};
}

RegistryCounts copy_counts(const gore_as_capture_registry_counts_v1& counts) noexcept {
  return RegistryCounts{
      counts.types,
      counts.functions,
      counts.object_properties,
      counts.global_properties,
      counts.enum_values,
      counts.funcdefs,
      counts.typedefs,
      counts.total_registrations,
  };
}

bool request_path(
    const wchar_t* characters,
    const std::uint32_t count,
    std::filesystem::path& output) {
  if (characters == nullptr || count == 0 || count > kMaxPathChars) {
    return false;
  }
  const std::wstring_view view(characters, count);
  if (std::find(view.begin(), view.end(), L'\0') != view.end()) {
    return false;
  }
  output = std::filesystem::path(std::wstring(view));
  return !output.empty();
}

template <typename Operation>
std::uint32_t with_session(const std::uint64_t session_id, Operation&& operation) noexcept {
  try {
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    if (!state.session || session_id == 0 || session_id != state.session_id) {
      return GORE_AS_CAPTURE_BRIDGE_INVALID_SESSION_V1;
    }
    return bridge_error(operation(*state.session));
  } catch (...) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
  }
}

}  // namespace

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_query_v1(
    gore_as_capture_bridge_contract_v1* const contract_out) {
  if (contract_out == nullptr) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  gore_as_capture_bridge_contract_v1 contract{};
  contract.struct_size = sizeof(contract);
  contract.abi_version = GORE_AS_CAPTURE_BRIDGE_ABI_V1;
  contract.hook_table_version = gore_as_capture::v1::kHookTableVersion;
  contract.hook_point_count =
      static_cast<std::uint32_t>(gore_as_capture::v1::kPinnedHookTable.size());
  contract.hook_table_fingerprint = gore_as_capture::v1::kPinnedHookTableFingerprint;
  contract.steam_app_id = gore_as_capture::v1::kSteamAppId;
  contract.steam_build_id = kExpectedObservedBuildId;
  contract.executable_bytes = gore_as_capture::v1::kExecutableBytes;
  contract.pe_size_of_image = gore_as_capture::v1::kPeSizeOfImage;
  contract.codeview_age = gore_as_capture::v1::kCodeViewAge;
  std::memcpy(
      contract.executable_sha256,
      gore_as_capture::v1::kExecutableSha256.data(),
      sizeof(contract.executable_sha256));
  std::memcpy(
      contract.codeview_guid_rsds,
      gore_as_capture::v1::kCodeViewGuidRsds.data(),
      sizeof(contract.codeview_guid_rsds));
  contract.test_fixture_only = kTestFixtureOnly;
  *contract_out = contract;
  return GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_hook_point_v1(
    const std::uint32_t index,
    gore_as_capture_hook_point_v1* const point_out) {
  if (point_out == nullptr || index >= gore_as_capture::v1::kPinnedHookTable.size()) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  const auto& point = gore_as_capture::v1::kPinnedHookTable[index];
  *point_out = gore_as_capture_hook_point_v1{
      static_cast<std::uint32_t>(point.kind), point.image_rva};
  return GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_attach_v1(
    const gore_as_capture_attach_request_v1* const request,
    std::uint64_t* const session_id_out) {
  if (request == nullptr || session_id_out == nullptr ||
      request->struct_size != sizeof(*request) ||
      request->abi_version != GORE_AS_CAPTURE_BRIDGE_ABI_V1 ||
      request->hook_table_version != gore_as_capture::v1::kHookTableVersion ||
      request->hook_table_fingerprint != gore_as_capture::v1::kPinnedHookTableFingerprint ||
      request->observed_steam_build_id != kExpectedObservedBuildId ||
      request->primary_image_base == 0 || request->reserved0 != 0 || request->reserved1 != 0 ||
      request->reserved2 != 0 || request->reserved3 != 0) {
    return GORE_AS_CAPTURE_BRIDGE_ABI_MISMATCH_V1;
  }
  if (reinterpret_cast<HMODULE>(request->primary_image_base) != GetModuleHandleW(nullptr)) {
    return GORE_AS_CAPTURE_BRIDGE_WRONG_TARGET_V1;
  }
  try {
    std::filesystem::path executable;
    std::filesystem::path output;
    if (!request_path(request->executable_path, request->executable_path_chars, executable) ||
        !request_path(request->output_path, request->output_path_chars, output)) {
      return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
    }
    GuidBytes capture_id{};
    std::memcpy(capture_id.data(), request->capture_id, capture_id.size());
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    if (state.session || state.unload_preparing || state.unload_prepared) {
      return GORE_AS_CAPTURE_BRIDGE_BUSY_V1;
    }
    auto session = std::make_unique<CaptureSession>();
    const auto open_error = session->open_pinned(
        executable,
        output,
        reinterpret_cast<const void*>(request->primary_image_base),
        request->observed_steam_build_id,
        capture_id);
    if (open_error != CaptureError::ok) {
      return bridge_error(open_error);
    }
    const std::uint64_t generation = state.next_generation++;
    state.session_id = kSessionSeed ^ (generation << 1u) ^ GetCurrentProcessId();
    if (state.session_id == 0) {
      state.session_id = kSessionSeed;
    }
    state.owner_thread = GetCurrentThreadId();
    state.primary_image = request->primary_image_base;
    state.session = std::move(session);
    *session_id_out = state.session_id;
    return GORE_AS_CAPTURE_BRIDGE_OK_V1;
  } catch (const std::bad_alloc&) {
    return GORE_AS_CAPTURE_BRIDGE_LIMIT_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_engine_property_v1(
    const std::uint64_t session_id,
    const std::uint32_t property_id,
    const std::uint64_t value,
    const std::uint32_t observation_rva) {
  return with_session(session_id, [&](CaptureSession& session) {
    return session.append_engine_property(property_id, value, observation_rva);
  });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_intern_primary_image_pointer_v1(
    const std::uint64_t session_id,
    const std::uintptr_t pointer,
    std::uint32_t* const token_out) {
  if (pointer == 0 || token_out == nullptr) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  return with_session(session_id, [&](CaptureSession& session) {
    return session.intern_primary_image_pointer(reinterpret_cast<const void*>(pointer), *token_out);
  });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_bind_begin_v1(
    const std::uint64_t session_id,
    const std::uint32_t callback_ordinal,
    const std::int32_t bind_order,
    const std::uint32_t callback_pointer_token,
    const gore_as_capture_registry_counts_v1* const counts,
    const std::uint8_t registry_sha256[32]) {
  if (counts == nullptr || registry_sha256 == nullptr) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  const RegistryCounts copied_counts = copy_counts(*counts);
  Digest digest{};
  std::memcpy(digest.data(), registry_sha256, digest.size());
  return with_session(session_id, [&](CaptureSession& session) {
    return session.append_bind_begin(
        callback_ordinal, bind_order, callback_pointer_token, copied_counts, digest);
  });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_bind_end_v1(
    const std::uint64_t session_id,
    const std::uint32_t callback_ordinal,
    const std::int32_t bind_order,
    const std::uint32_t callback_pointer_token,
    const gore_as_capture_registry_counts_v1* const counts,
    const std::uint8_t registry_sha256[32]) {
  if (counts == nullptr || registry_sha256 == nullptr) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  const RegistryCounts copied_counts = copy_counts(*counts);
  Digest digest{};
  std::memcpy(digest.data(), registry_sha256, digest.size());
  return with_session(session_id, [&](CaptureSession& session) {
    return session.append_bind_end(
        callback_ordinal, bind_order, callback_pointer_token, copied_counts, digest);
  });
}

#define GORE_AS_CAPTURE_JSON_EXPORT(function_name, method_name)                             \
  extern "C" std::uint32_t GORE_AS_CAPTURE_CALL function_name(                             \
      const std::uint64_t session_id,                                                       \
      const std::uint8_t* const json,                                                       \
      const std::uint32_t json_bytes) {                                                     \
    if (json == nullptr || json_bytes == 0) {                                               \
      return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;                                   \
    }                                                                                       \
    return with_session(session_id, [&](CaptureSession& session) {                          \
      return session.method_name(json_span(json, json_bytes));                              \
    });                                                                                     \
  }

GORE_AS_CAPTURE_JSON_EXPORT(
    gore_as_capture_bridge_append_registry_delta_json_v1, append_registry_delta_json)
GORE_AS_CAPTURE_JSON_EXPORT(
    gore_as_capture_bridge_append_post_bind_mutation_json_v1, append_post_bind_mutation_json)
GORE_AS_CAPTURE_JSON_EXPORT(
    gore_as_capture_bridge_append_registry_support_json_v1, append_registry_support_json)
GORE_AS_CAPTURE_JSON_EXPORT(
    gore_as_capture_bridge_append_final_post_bind_state_json_v1,
    append_final_post_bind_state_json)

#undef GORE_AS_CAPTURE_JSON_EXPORT

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_append_build_jit_v1(
    const std::uint64_t session_id,
    const gore_as_capture_build_jit_v1* const fact) {
  if (fact == nullptr || fact->struct_size != sizeof(*fact)) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  BuildJitFact copied{};
  copied.build_identifier = fact->build_identifier;
  copied.shipping_cache_matches = fact->shipping_cache_matches != 0;
  copied.jit_info_present = fact->jit_info_present != 0;
  copied.jit_guid_matches = fact->jit_guid_matches != 0;
  copied.jit_database_cleared = fact->jit_database_cleared != 0;
  copied.as_reference_debugging = fact->as_reference_debugging != 0;
  copied.fork_opcode_table_201_212_present =
      fact->fork_opcode_table_201_212_present != 0;
  copied.reference_debug_opcodes_emittable =
      fact->reference_debug_opcodes_emittable != 0;
  copied.resolve_object_ptr_callback_registered =
      fact->resolve_object_ptr_callback_registered != 0;
  std::memcpy(copied.precompiled_guid.data(), fact->precompiled_guid, copied.precompiled_guid.size());
  std::memcpy(
      copied.compiled_jit_guid.data(),
      fact->compiled_jit_guid,
      copied.compiled_jit_guid.size());
  copied.get_build_identifier_rva = fact->get_build_identifier_rva;
  copied.get_static_jit_info_rva = fact->get_static_jit_info_rva;
  return with_session(
      session_id, [&](CaptureSession& session) { return session.append_build_jit(copied); });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_frontend_config_json_v1(
    const std::uint64_t session_id,
    const std::uint32_t config_kind,
    const std::uint8_t* const json,
    const std::uint32_t json_bytes) {
  if (json == nullptr || json_bytes == 0) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  return with_session(session_id, [&](CaptureSession& session) {
    return session.append_frontend_config_json(config_kind, json_span(json, json_bytes));
  });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_append_frontend_boundary_v1(
    const std::uint64_t session_id,
    const gore_as_capture_frontend_boundary_v1* const boundary) {
  if (boundary == nullptr || boundary->struct_size != sizeof(*boundary) || boundary->kind < 1 ||
      boundary->kind > 4) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  FrontendBoundary copied{};
  copied.kind = static_cast<FrontendBoundaryKind>(boundary->kind);
  copied.observation_rva = boundary->observation_rva;
  copied.module_count = boundary->module_count;
  copied.result_code = boundary->result_code;
  std::memcpy(copied.config_sha256.data(), boundary->config_sha256, copied.config_sha256.size());
  std::memcpy(copied.input_sha256.data(), boundary->input_sha256, copied.input_sha256.size());
  std::memcpy(copied.output_sha256.data(), boundary->output_sha256, copied.output_sha256.size());
  return with_session(session_id, [&](CaptureSession& session) {
    return session.append_frontend_boundary(copied);
  });
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_seal_and_detach_v1(const std::uint64_t session_id) {
  try {
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    if (!state.session || session_id == 0 || session_id != state.session_id) {
      return GORE_AS_CAPTURE_BRIDGE_INVALID_SESSION_V1;
    }
    if (state.owner_thread != GetCurrentThreadId()) {
      return GORE_AS_CAPTURE_BRIDGE_WRONG_THREAD_V1;
    }
    const CaptureError seal_error = state.session->seal();
    if (seal_error != CaptureError::ok) {
      return bridge_error(seal_error);
    }
    state.session.reset();
    state.session_id = 0;
    state.owner_thread = 0;
    state.primary_image = 0;
    return GORE_AS_CAPTURE_BRIDGE_OK_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_bridge_abort_and_detach_v1(const std::uint64_t session_id) {
  try {
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    if (!state.session || session_id == 0 || session_id != state.session_id) {
      return GORE_AS_CAPTURE_BRIDGE_INVALID_SESSION_V1;
    }
    if (state.owner_thread != GetCurrentThreadId()) {
      return GORE_AS_CAPTURE_BRIDGE_WRONG_THREAD_V1;
    }
    state.session.reset();
    state.session_id = 0;
    state.owner_thread = 0;
    state.primary_image = 0;
    return GORE_AS_CAPTURE_BRIDGE_OK_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_bridge_prepare_unload_v1() {
  try {
    // Successful preparation is a terminal lease: once published, no later bridge attach or
    // instrumentation preflight may enter. Check instrumentation on both sides of publishing
    // that lease so an install racing between the checks either observes the lease and refuses,
    // or becomes visible to the second check and makes this attempt BUSY.
    if (gore_as_capture_instrumentation_prepare_unload_v1() !=
        GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) {
      return GORE_AS_CAPTURE_BRIDGE_BUSY_V1;
    }
    {
      auto& state = bridge_state();
      std::scoped_lock lock(state.mutex);
      if (state.session || state.unload_preparing) {
        return GORE_AS_CAPTURE_BRIDGE_BUSY_V1;
      }
      if (state.unload_prepared) return GORE_AS_CAPTURE_BRIDGE_OK_V1;
      state.unload_preparing = true;
    }
    const bool safe = gore_as_capture_instrumentation_prepare_unload_v1() ==
                      GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
    {
      auto& state = bridge_state();
      std::scoped_lock lock(state.mutex);
      state.unload_preparing = false;
      state.unload_prepared = safe;
    }
    return safe ? GORE_AS_CAPTURE_BRIDGE_OK_V1 : GORE_AS_CAPTURE_BRIDGE_BUSY_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_STATE_V1;
  }
}

bool gore_as_capture::v1::instrumentation::bridge_validate_live_session_v1(
    const std::uint64_t session_id,
    const std::uintptr_t primary_image) noexcept {
  try {
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    return !state.unload_preparing && !state.unload_prepared && state.session != nullptr &&
           session_id != 0 && state.session_id == session_id &&
           primary_image != 0 && state.primary_image == primary_image &&
           state.owner_thread == GetCurrentThreadId() &&
           state.session->status() == CaptureError::ok;
  } catch (...) {
    return false;
  }
}

bool gore_as_capture::v1::instrumentation::bridge_adopt_runtime_owner_v1(
    const std::uint64_t session_id,
    const std::uintptr_t primary_image) noexcept {
  try {
    auto& state = bridge_state();
    std::scoped_lock lock(state.mutex);
    if (state.unload_preparing || state.unload_prepared || !state.session ||
        session_id == 0 || state.session_id != session_id || primary_image == 0 ||
        state.primary_image != primary_image ||
        state.session->status() != CaptureError::ok) {
      return false;
    }
    state.owner_thread = GetCurrentThreadId();
    return state.owner_thread != 0;
  } catch (...) {
    return false;
  }
}

BOOL WINAPI DllMain(const HINSTANCE module, const DWORD reason, LPVOID) {
  if (reason == DLL_PROCESS_ATTACH) {
    DisableThreadLibraryCalls(module);
  }
  // No target inspection, hook installation, allocation, I/O, or synchronization under the
  // loader lock. The host must use attach and prepare_unload explicitly.
  return TRUE;
}
