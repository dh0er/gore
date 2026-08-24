#include "gore_as_capture/bridge.h"
#include "gore_as_capture/format.hpp"
#include "gore_as_capture/hook_table.hpp"
#include "gore_as_capture/instrumentation.h"
#include "gore_as_capture/instrumentation.hpp"
#include "materializer.hpp"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <span>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

namespace {

template <typename Function>
Function load_export(const HMODULE module, const char* name) noexcept {
  const FARPROC address = GetProcAddress(module, name);
  Function function = nullptr;
  static_assert(sizeof(function) == sizeof(address));
  std::memcpy(&function, &address, sizeof(function));
  return function;
}

struct BridgeApi final {
  decltype(&gore_as_capture_bridge_query_v1) query{};
  decltype(&gore_as_capture_bridge_hook_point_v1) hook_point{};
  decltype(&gore_as_capture_bridge_attach_v1) attach{};
  decltype(&gore_as_capture_bridge_append_engine_property_v1) engine_property{};
  decltype(&gore_as_capture_bridge_intern_primary_image_pointer_v1) intern_pointer{};
  decltype(&gore_as_capture_bridge_append_bind_begin_v1) bind_begin{};
  decltype(&gore_as_capture_bridge_append_bind_end_v1) bind_end{};
  decltype(&gore_as_capture_bridge_append_registry_delta_json_v1) registry_delta{};
  decltype(&gore_as_capture_bridge_append_post_bind_mutation_json_v1) post_bind_mutation{};
  decltype(&gore_as_capture_bridge_append_registry_support_json_v1) registry_support{};
  decltype(&gore_as_capture_bridge_append_final_post_bind_state_json_v1) final_state{};
  decltype(&gore_as_capture_bridge_append_build_jit_v1) build_jit{};
  decltype(&gore_as_capture_bridge_append_frontend_config_json_v1) frontend_config{};
  decltype(&gore_as_capture_bridge_append_frontend_boundary_v1) frontend_boundary{};
  decltype(&gore_as_capture_bridge_seal_and_detach_v1) seal_and_detach{};
  decltype(&gore_as_capture_bridge_abort_and_detach_v1) abort_and_detach{};
  decltype(&gore_as_capture_bridge_prepare_unload_v1) prepare_unload{};
  decltype(&gore_as_capture_instrumentation_query_v1) instrumentation_query{};
  decltype(&gore_as_capture_instrumentation_query_site_contract_v1)
      instrumentation_site_contract{};
  decltype(&gore_as_capture_instrumentation_query_registration_hook_set_v1)
      instrumentation_registration_hook_set{};
  decltype(&gore_as_capture_instrumentation_query_registration_site_v1)
      instrumentation_registration_site{};
  decltype(&gore_as_capture_instrumentation_validate_current_image_v1)
      instrumentation_validate{};
  decltype(&gore_as_capture_instrumentation_install_v1) instrumentation_install{};
  decltype(&gore_as_capture_instrumentation_uninstall_v1) instrumentation_uninstall{};
  decltype(&gore_as_capture_instrumentation_prepare_unload_v1)
      instrumentation_prepare_unload{};
  decltype(&gore_as_capture_instrumentation_observe_set_engine_property_v1)
      instrumented_engine_property{};
  decltype(&gore_as_capture_instrumentation_synthetic_selftest_v1)
      instrumentation_selftest{};

  [[nodiscard]] bool complete() const noexcept {
    return query != nullptr && hook_point != nullptr && attach != nullptr &&
           engine_property != nullptr && intern_pointer != nullptr && bind_begin != nullptr &&
           bind_end != nullptr && registry_delta != nullptr && post_bind_mutation != nullptr &&
           registry_support != nullptr && final_state != nullptr && build_jit != nullptr &&
           frontend_config != nullptr && frontend_boundary != nullptr &&
           seal_and_detach != nullptr && abort_and_detach != nullptr &&
           prepare_unload != nullptr && instrumentation_query != nullptr &&
           instrumentation_site_contract != nullptr &&
           instrumentation_registration_hook_set != nullptr &&
           instrumentation_registration_site != nullptr &&
           instrumentation_validate != nullptr && instrumentation_install != nullptr &&
           instrumentation_uninstall != nullptr &&
           instrumentation_prepare_unload != nullptr &&
           instrumented_engine_property != nullptr && instrumentation_selftest != nullptr;
  }
};

BridgeApi load_api(const HMODULE module) noexcept {
  BridgeApi api;
#define GORE_AS_CAPTURE_LOAD(field, symbol) api.field = load_export<decltype(api.field)>(module, #symbol)
  GORE_AS_CAPTURE_LOAD(query, gore_as_capture_bridge_query_v1);
  GORE_AS_CAPTURE_LOAD(hook_point, gore_as_capture_bridge_hook_point_v1);
  GORE_AS_CAPTURE_LOAD(attach, gore_as_capture_bridge_attach_v1);
  GORE_AS_CAPTURE_LOAD(engine_property, gore_as_capture_bridge_append_engine_property_v1);
  GORE_AS_CAPTURE_LOAD(
      intern_pointer, gore_as_capture_bridge_intern_primary_image_pointer_v1);
  GORE_AS_CAPTURE_LOAD(bind_begin, gore_as_capture_bridge_append_bind_begin_v1);
  GORE_AS_CAPTURE_LOAD(bind_end, gore_as_capture_bridge_append_bind_end_v1);
  GORE_AS_CAPTURE_LOAD(
      registry_delta, gore_as_capture_bridge_append_registry_delta_json_v1);
  GORE_AS_CAPTURE_LOAD(
      post_bind_mutation, gore_as_capture_bridge_append_post_bind_mutation_json_v1);
  GORE_AS_CAPTURE_LOAD(
      registry_support, gore_as_capture_bridge_append_registry_support_json_v1);
  GORE_AS_CAPTURE_LOAD(
      final_state, gore_as_capture_bridge_append_final_post_bind_state_json_v1);
  GORE_AS_CAPTURE_LOAD(build_jit, gore_as_capture_bridge_append_build_jit_v1);
  GORE_AS_CAPTURE_LOAD(
      frontend_config, gore_as_capture_bridge_append_frontend_config_json_v1);
  GORE_AS_CAPTURE_LOAD(
      frontend_boundary, gore_as_capture_bridge_append_frontend_boundary_v1);
  GORE_AS_CAPTURE_LOAD(seal_and_detach, gore_as_capture_bridge_seal_and_detach_v1);
  GORE_AS_CAPTURE_LOAD(abort_and_detach, gore_as_capture_bridge_abort_and_detach_v1);
  GORE_AS_CAPTURE_LOAD(prepare_unload, gore_as_capture_bridge_prepare_unload_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_query, gore_as_capture_instrumentation_query_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_site_contract,
      gore_as_capture_instrumentation_query_site_contract_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_registration_hook_set,
      gore_as_capture_instrumentation_query_registration_hook_set_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_registration_site,
      gore_as_capture_instrumentation_query_registration_site_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_validate,
      gore_as_capture_instrumentation_validate_current_image_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_install, gore_as_capture_instrumentation_install_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_uninstall, gore_as_capture_instrumentation_uninstall_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_prepare_unload,
      gore_as_capture_instrumentation_prepare_unload_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumented_engine_property,
      gore_as_capture_instrumentation_observe_set_engine_property_v1);
  GORE_AS_CAPTURE_LOAD(
      instrumentation_selftest,
      gore_as_capture_instrumentation_synthetic_selftest_v1);
#undef GORE_AS_CAPTURE_LOAD
  return api;
}

class LoadedBridge final {
 public:
  explicit LoadedBridge(const std::filesystem::path& path) noexcept
      : module_(LoadLibraryExW(path.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                        LOAD_LIBRARY_SEARCH_SYSTEM32)) {}
  ~LoadedBridge() {
    if (module_ != nullptr) {
      FreeLibrary(module_);
    }
  }
  LoadedBridge(const LoadedBridge&) = delete;
  LoadedBridge& operator=(const LoadedBridge&) = delete;
  [[nodiscard]] HMODULE get() const noexcept { return module_; }

 private:
  HMODULE module_{};
};

class TempTree final {
 public:
  TempTree() {
    std::array<wchar_t, 32768> root{};
    const DWORD length = GetTempPathW(static_cast<DWORD>(root.size()), root.data());
    if (length == 0 || length >= root.size()) {
      return;
    }
    for (std::uint32_t attempt = 0; attempt < 128; ++attempt) {
      path_ = std::filesystem::path(root.data()) /
              (L"gore-as-capture-offline-e2e-" + std::to_wstring(GetCurrentProcessId()) + L"-" +
               std::to_wstring(GetTickCount64()) + L"-" + std::to_wstring(attempt));
      if (CreateDirectoryW(path_.c_str(), nullptr) != FALSE) {
        valid_ = true;
        break;
      }
      if (GetLastError() != ERROR_ALREADY_EXISTS) {
        break;
      }
    }
  }
  ~TempTree() {
    if (valid_) {
      std::error_code ignored;
      std::filesystem::remove_all(path_, ignored);
    }
  }
  [[nodiscard]] bool valid() const noexcept { return valid_; }
  [[nodiscard]] const std::filesystem::path& path() const noexcept { return path_; }

 private:
  std::filesystem::path path_;
  bool valid_{};
};

bool expect(const bool condition, const char* message) {
  if (!condition) {
    std::cerr << "FAILED: " << message << '\n';
  }
  return condition;
}

std::filesystem::path current_executable() {
  std::array<wchar_t, 32768> path{};
  const DWORD length =
      GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
  if (length == 0 || length >= path.size()) {
    return {};
  }
  return std::filesystem::path(path.data(), path.data() + length);
}

gore_as_capture_attach_request_v1 attach_request(
    const gore_as_capture_bridge_contract_v1& contract,
    const std::filesystem::path& executable,
    const std::filesystem::path& output) {
  gore_as_capture_attach_request_v1 request{};
  request.struct_size = sizeof(request);
  request.abi_version = GORE_AS_CAPTURE_BRIDGE_ABI_V1;
  request.hook_table_version = contract.hook_table_version;
  request.hook_table_fingerprint = contract.hook_table_fingerprint;
  request.observed_steam_build_id = contract.steam_build_id;
  request.primary_image_base = reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr));
  request.executable_path = executable.c_str();
  request.executable_path_chars = static_cast<std::uint32_t>(executable.native().size());
  request.output_path = output.c_str();
  request.output_path_chars = static_cast<std::uint32_t>(output.native().size());
  std::fill(std::begin(request.capture_id), std::end(request.capture_id), std::uint8_t{0x42});
  return request;
}

bool append_boundary(
    const BridgeApi& api,
    const std::uint64_t session,
    const std::uint32_t kind,
    const std::uint32_t rva,
    const std::uint32_t module_count) {
  gore_as_capture_frontend_boundary_v1 boundary{};
  boundary.struct_size = sizeof(boundary);
  boundary.kind = kind;
  boundary.observation_rva = rva;
  boundary.module_count = module_count;
  std::fill(
      std::begin(boundary.config_sha256),
      std::end(boundary.config_sha256),
      std::uint8_t{0x51});
  if (kind != 3) {
    std::fill(
        std::begin(boundary.input_sha256),
        std::end(boundary.input_sha256),
        std::uint8_t{0x61});
  }
  if (kind != 1 && kind != 3) {
    std::fill(
        std::begin(boundary.output_sha256),
        std::end(boundary.output_sha256),
        std::uint8_t{0x62});
  }
  return api.frontend_boundary(session, &boundary) == GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

bool write_complete_capture(
    const BridgeApi& api,
    const gore_as_capture_bridge_contract_v1& contract,
    const std::filesystem::path& executable,
    const std::filesystem::path& output) {
  auto request = attach_request(contract, executable, output);
  std::uint64_t session = 0;
  if (api.attach(&request, &session) != GORE_AS_CAPTURE_BRIDGE_OK_V1 || session == 0) {
    return false;
  }
  const auto abort_on_failure = [&] { (void)api.abort_and_detach(session); };
  if (api.instrumented_engine_property(session, 2, 1) !=
      GORE_AS_CAPTURE_BRIDGE_OK_V1) {
    abort_on_failure();
    return false;
  }
  const auto image = request.primary_image_base;
  std::uint32_t callback_token = 0;
  std::uint32_t stub_token = 0;
  if (api.intern_pointer(session, image + 0x1000u, &callback_token) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.intern_pointer(session, image + 0x2000u, &stub_token) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      callback_token != 0 || stub_token != 1) {
    abort_on_failure();
    return false;
  }
  gore_as_capture_registry_counts_v1 before{};
  gore_as_capture_registry_counts_v1 after{};
  after.functions = 1;
  after.total_registrations = 1;
  std::array<std::uint8_t, 32> before_digest{};
  std::array<std::uint8_t, 32> after_digest{};
  before_digest.fill(0x31);
  after_digest.fill(0x32);
  constexpr std::array<std::uint8_t, 2> json{'{', '}'};
  if (api.bind_begin(session, 0, 10, callback_token, &before, before_digest.data()) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.registry_delta(session, json.data(), static_cast<std::uint32_t>(json.size())) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.post_bind_mutation(session, json.data(), static_cast<std::uint32_t>(json.size())) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.bind_end(session, 0, 10, callback_token, &after, after_digest.data()) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.registry_support(session, json.data(), static_cast<std::uint32_t>(json.size())) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1 ||
      api.final_state(session, json.data(), static_cast<std::uint32_t>(json.size())) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1) {
    abort_on_failure();
    return false;
  }
  gore_as_capture_build_jit_v1 build{};
  build.struct_size = sizeof(build);
  build.build_identifier = gore_as_capture::v1::kBuildIdentifier;
  build.shipping_cache_matches = 1;
  build.fork_opcode_table_201_212_present = 1;
  std::memcpy(
      build.precompiled_guid,
      gore_as_capture::v1::kPrecompiledGuid.data(),
      sizeof(build.precompiled_guid));
  build.get_build_identifier_rva = gore_as_capture::v1::kRvaGetBuildIdentifier;
  build.get_static_jit_info_rva = gore_as_capture::v1::kRvaGetStaticJitInfo;
  if (api.build_jit(session, &build) != GORE_AS_CAPTURE_BRIDGE_OK_V1) {
    abort_on_failure();
    return false;
  }
  for (std::uint32_t kind = 1; kind <= 3; ++kind) {
    if (api.frontend_config(
            session, kind, json.data(), static_cast<std::uint32_t>(json.size())) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) {
      abort_on_failure();
      return false;
    }
  }
  if (!append_boundary(
          api,
          session,
          1,
          gore_as_capture::v1::kRvaInitialCompileEnter,
          0) ||
      !append_boundary(
          api,
          session,
          3,
          gore_as_capture::v1::kRvaPreprocessorConstructed,
          0) ||
      !append_boundary(
          api,
          session,
          4,
          gore_as_capture::v1::kRvaInitialCompileReturn,
          1)) {
    abort_on_failure();
    return false;
  }
  return api.seal_and_detach(session) == GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

std::vector<std::byte> read_file(const std::filesystem::path& path) {
  std::ifstream input(path, std::ios::binary);
  const std::vector<char> characters(
      (std::istreambuf_iterator<char>(input)), std::istreambuf_iterator<char>());
  std::vector<std::byte> bytes(characters.size());
  if (!characters.empty()) {
    std::memcpy(bytes.data(), characters.data(), characters.size());
  }
  return bytes;
}

bool write_new_file(
    const std::filesystem::path& path,
    std::span<const std::byte> bytes) noexcept {
  const HANDLE file = CreateFileW(
      path.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
  if (file == INVALID_HANDLE_VALUE) {
    return false;
  }
  bool ok = true;
  while (!bytes.empty()) {
    const DWORD request =
        static_cast<DWORD>(std::min<std::size_t>(bytes.size(), 1u << 30u));
    DWORD written = 0;
    if (WriteFile(file, bytes.data(), request, &written, nullptr) == FALSE || written != request) {
      ok = false;
      break;
    }
    bytes = bytes.subspan(written);
  }
  ok = ok && FlushFileBuffers(file) != FALSE;
  ok = CloseHandle(file) != FALSE && ok;
  return ok;
}

int run_synthetic_e2e(
    const std::filesystem::path& bridge_path,
    const std::filesystem::path& production_bridge_path) {
  TempTree tree;
  if (!expect(tree.valid(), "create isolated output tree")) {
    return 1;
  }
  const auto executable = current_executable();
  if (!expect(!executable.empty(), "resolve current fake-target executable")) {
    return 1;
  }
  {
    LoadedBridge production(production_bridge_path);
    if (!expect(production.get() != nullptr, "load production bridge for offline contract audit")) {
      return 1;
    }
    const BridgeApi production_api = load_api(production.get());
    gore_as_capture_bridge_contract_v1 production_contract{};
    gore_as_capture_instrumentation_contract_v1 production_instrumentation{};
    gore_as_capture_registration_hook_set_v1 production_registration{};
    gore_as_capture_instrumentation_selftest_v1 forbidden_selftest{};
    if (!expect(
            production_api.complete() &&
                production_api.query(&production_contract) == GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
                production_contract.test_fixture_only == 0 &&
                production_contract.steam_build_id == gore_as_capture::v1::kSteamBuildId &&
                production_contract.hook_table_fingerprint ==
                    gore_as_capture::v1::kPinnedHookTableFingerprint &&
                production_api.instrumentation_query(&production_instrumentation) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
                production_instrumentation.production_installable == 1 &&
                production_instrumentation.statically_extractable_hook_mask ==
                    gore_as_capture::v1::instrumentation::kAllHookMask &&
                production_instrumentation.unresolved_hook_mask ==
                    gore_as_capture::v1::instrumentation::kUnresolvedHookMask &&
                production_instrumentation.prolog_table_fingerprint ==
                    gore_as_capture::v1::instrumentation::kPinnedPrologTableFingerprint &&
                production_api.instrumentation_registration_hook_set(
                    &production_registration) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
                production_registration.hook_count == 14 &&
                production_registration.statically_closed_hook_mask ==
                    gore_as_capture::v1::instrumentation::registration::
                        kAllRegistrationHookMask &&
                production_registration.unresolved_hook_mask == 0 &&
                production_registration.production_installable == 1 &&
                production_api.instrumentation_validate(
                    reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr))) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1 &&
                production_api.instrumentation_install(
                    1, reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr))) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1 &&
                production_api.instrumentation_selftest(&forbidden_selftest) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1 &&
                production_api.instrumentation_prepare_unload() ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1,
            "production DLL exposes the exact pinned but release-gated instrumentation contract")) {
      return 1;
    }
    const auto refused = tree.path() / L"production-refused.capture";
    auto request = attach_request(production_contract, executable, refused);
    std::uint64_t session = 0;
    if (!expect(
            production_api.attach(&request, &session) ==
                    GORE_AS_CAPTURE_BRIDGE_WRONG_TARGET_V1 &&
                session == 0 && !std::filesystem::exists(refused) &&
                production_api.prepare_unload() == GORE_AS_CAPTURE_BRIDGE_OK_V1,
            "production target pin rejects the synthetic image before output creation")) {
      return 1;
    }
  }
  LoadedBridge loaded(bridge_path);
  if (!expect(loaded.get() != nullptr, "load the test-only bridge into this process")) {
    return 1;
  }
  const BridgeApi api = load_api(loaded.get());
  if (!expect(api.complete(), "resolve the complete bridge ABI")) {
    return 1;
  }
  gore_as_capture_bridge_contract_v1 contract{};
  gore_as_capture_instrumentation_contract_v1 instrumentation_contract{};
  gore_as_capture_instrumentation_selftest_v1 instrumentation_selftest{};
  gore_as_capture_registration_hook_set_v1 registration_hook_set{};
  const auto instrumentation_selftest_status =
      api.instrumentation_selftest(&instrumentation_selftest);
  if (instrumentation_selftest_status != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) {
    std::cerr << "instrumentation selftest status=" << instrumentation_selftest_status
              << " static_mask=0x" << std::hex << instrumentation_selftest.reserved0
              << std::dec << '\n';
  }
  if (!expect(
          api.query(&contract) == GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
              contract.struct_size == sizeof(contract) && contract.test_fixture_only == 1 &&
              contract.hook_point_count == gore_as_capture::v1::kPinnedHookTable.size() &&
              contract.hook_table_fingerprint ==
                  gore_as_capture::v1::kPinnedHookTableFingerprint &&
              api.instrumentation_query(&instrumentation_contract) ==
                  GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              instrumentation_contract.test_fixture_only == 1 &&
              instrumentation_contract.production_installable == 0 &&
              instrumentation_contract.hook_point_count == 9 &&
              api.instrumentation_registration_hook_set(&registration_hook_set) ==
                  GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              registration_hook_set.struct_size == sizeof(registration_hook_set) &&
              registration_hook_set.contract_version ==
                  gore_as_capture::v1::instrumentation::registration::kContractVersion &&
              registration_hook_set.hook_count == 14 &&
              registration_hook_set.engine_vtable_rva ==
                  gore_as_capture::v1::instrumentation::registration::kEngineVtableRva &&
              registration_hook_set.table_fingerprint ==
                  gore_as_capture::v1::instrumentation::registration::
                      kRegistrationTableFingerprint &&
              registration_hook_set.prolog_fingerprint ==
                  gore_as_capture::v1::instrumentation::registration::
                      kRegistrationPrologFingerprint &&
              registration_hook_set.production_installable == 0 &&
              instrumentation_selftest_status == GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              instrumentation_selftest.installed_all_nine != 0 &&
              instrumentation_selftest.restored_all_nine != 0 &&
              instrumentation_selftest.prolog_drift_refused_without_write != 0 &&
              instrumentation_selftest.injected_failure_rolled_back != 0 &&
              instrumentation_selftest.wrong_thread_refused != 0 &&
              instrumentation_selftest.unload_while_installed_refused != 0 &&
              instrumentation_selftest.record_order_exact != 0 &&
              instrumentation_selftest.record_order_drift_refused != 0 &&
              instrumentation_selftest.reserved0 ==
                  GORE_AS_CAPTURE_SELFTEST_STATIC_RE_ALL_V1 &&
              api.instrumentation_install(
                  1, reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr))) ==
                  GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1 &&
              api.instrumentation_prepare_unload() ==
                  GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1,
          "query and exercise the exact fake-target instrumentation contract")) {
    return 1;
  }
  for (std::uint32_t index = 0; index < contract.hook_point_count; ++index) {
    gore_as_capture_hook_point_v1 point{};
    gore_as_capture_instrumentation_site_contract_v1 site{};
    const auto& expected = gore_as_capture::v1::kPinnedHookTable[index];
    const auto& expected_span =
        gore_as_capture::v1::instrumentation::kPinnedInstructionSpans[index];
    const auto& expected_site =
        gore_as_capture::v1::instrumentation::kStaticSiteContracts[index];
    if (!expect(
            api.hook_point(index, &point) == GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
                point.kind == static_cast<std::uint32_t>(expected.kind) &&
                point.image_rva == expected.image_rva &&
                api.instrumentation_site_contract(index, &site) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
                site.struct_size == sizeof(site) && site.index == index &&
                site.hook_kind == point.kind && site.observation_rva == point.image_rva &&
                site.patch_anchor_rva == expected_span.patch_anchor_rva &&
                site.overwrite_bytes == expected_span.byte_count &&
                site.continuation_rva ==
                    expected_span.patch_anchor_rva + expected_span.byte_count &&
                site.transfer_kind == expected_site.transfer_kind &&
                site.frame_kind == expected_site.frame_kind &&
                site.register_read_mask == expected_site.register_read_mask &&
                site.direct_callee_rva == expected_site.direct_callee_rva,
            "bridge exposes the exact hook and static frame/transfer contracts")) {
      return 1;
    }
  }
  for (std::uint32_t index = 0; index < registration_hook_set.hook_count; ++index) {
    gore_as_capture_registration_site_contract_v1 site{};
    const auto& expected =
        gore_as_capture::v1::instrumentation::registration::kPinnedRegistrationHooks[index];
    if (!expect(
            api.instrumentation_registration_site(index, &site) ==
                    GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
                site.struct_size == sizeof(site) && site.index == index &&
                site.registration_kind == expected.kind &&
                site.vtable_slot == expected.vtable_slot &&
                site.function_rva == expected.function_rva &&
                site.overwrite_bytes == expected.overwrite_bytes &&
                site.continuation_rva == expected.function_rva + expected.overwrite_bytes &&
                site.generated_unwind_operation_count ==
                    expected.unwind_operation_count &&
                site.argument_count == expected.argument_count &&
                site.return_source == GORE_AS_CAPTURE_REGISTRATION_RETURN_EAX_I32_V1 &&
                std::memcmp(
                    site.expected_prolog,
                    expected.expected.data(),
                    expected.overwrite_bytes) == 0,
            "bridge exposes the exact 14-entry central registration contract")) {
      return 1;
    }
  }

  const auto capture_one = tree.path() / L"one.capture";
  const auto capture_two = tree.path() / L"two.capture";
  const auto summary_one = tree.path() / L"one.summary.json";
  const auto summary_two = tree.path() / L"two.summary.json";
  if (!expect(
          write_complete_capture(api, contract, executable, capture_one),
          "capture and seal first deterministic stream") ||
      !expect(
          write_complete_capture(api, contract, executable, capture_two),
          "capture and seal second deterministic stream")) {
    return 1;
  }
  if (!expect(read_file(capture_one) == read_file(capture_two), "capture output is deterministic")) {
    return 1;
  }

  auto collision_request = attach_request(contract, executable, capture_one);
  std::uint64_t collision_session = 0;
  if (!expect(
          api.attach(&collision_request, &collision_session) ==
                  GORE_AS_CAPTURE_BRIDGE_OUTPUT_EXISTS_V1 &&
              collision_session == 0,
          "CREATE_NEW preserves an existing capture")) {
    return 1;
  }

  const auto materialized_one =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(capture_one, summary_one);
  const auto materialized_two =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(capture_two, summary_two);
  if (!expect(
          materialized_one.error == gore_as_capture::v1::offline::MaterializeError::ok &&
              materialized_two.error == gore_as_capture::v1::offline::MaterializeError::ok &&
              materialized_one.record_count == 16 &&
              materialized_one.sealed_stream_sha256 == materialized_two.sealed_stream_sha256 &&
              read_file(summary_one) == read_file(summary_two),
          "materialize identical wire summaries")) {
    return 1;
  }
  const auto summary_collision =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(capture_one, summary_one);
  if (!expect(
          summary_collision.error ==
              gore_as_capture::v1::offline::MaterializeError::output_exists,
          "materializer never replaces an existing summary")) {
    return 1;
  }
  const auto named_stream_summary =
      std::filesystem::path(summary_one.native() + L":forbidden-stream");
  const auto named_stream_result =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(
          capture_one, named_stream_summary);
  if (!expect(
          named_stream_result.error ==
              gore_as_capture::v1::offline::MaterializeError::invalid_argument,
          "materializer refuses named-stream output")) {
    return 1;
  }

  auto corrupted_bytes = read_file(capture_one);
  if (!expect(
          corrupted_bytes.size() > gore_as_capture::v1::kHeaderBytes +
                                       gore_as_capture::v1::kRecordHeaderBytes,
          "capture has a mutable record fixture")) {
    return 1;
  }
  corrupted_bytes[gore_as_capture::v1::kHeaderBytes +
                  gore_as_capture::v1::kRecordHeaderBytes] ^= std::byte{1};
  const auto corrupted = tree.path() / L"corrupted.capture";
  const auto corrupted_summary = tree.path() / L"corrupted.summary.json";
  if (!expect(write_new_file(corrupted, corrupted_bytes), "write corrupt sealed-stream fixture")) {
    return 1;
  }
  const auto corrupt_result =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(
          corrupted, corrupted_summary);
  if (!expect(
          corrupt_result.error ==
                  gore_as_capture::v1::offline::MaterializeError::digest_mismatch &&
              !std::filesystem::exists(corrupted_summary),
          "materializer rejects digest drift before creating output")) {
    return 1;
  }

  const auto incomplete = tree.path() / L"incomplete.capture";
  auto incomplete_request = attach_request(contract, executable, incomplete);
  std::uint64_t incomplete_session = 0;
  if (!expect(
          api.attach(&incomplete_request, &incomplete_session) ==
                  GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
              api.engine_property(
                  incomplete_session, 2, 1, gore_as_capture::v1::kRvaSetEngineProperty) ==
                  GORE_AS_CAPTURE_BRIDGE_OK_V1,
          "start an intentionally incomplete stream")) {
    return 1;
  }
  std::uint32_t wrong_thread_result = GORE_AS_CAPTURE_BRIDGE_OK_V1;
  std::thread wrong_thread([&] {
    wrong_thread_result = api.abort_and_detach(incomplete_session);
  });
  wrong_thread.join();
  if (!expect(
          wrong_thread_result == GORE_AS_CAPTURE_BRIDGE_WRONG_THREAD_V1 &&
              api.abort_and_detach(incomplete_session) == GORE_AS_CAPTURE_BRIDGE_OK_V1,
          "detach is owner-thread-bound and abort retains an unsealed diagnostic")) {
    return 1;
  }
  const auto rejected_summary = tree.path() / L"incomplete.summary.json";
  const auto rejected =
      gore_as_capture::v1::offline::materialize_capture_summary_v1(incomplete, rejected_summary);
  if (!expect(
          rejected.error != gore_as_capture::v1::offline::MaterializeError::ok &&
              !std::filesystem::exists(rejected_summary),
          "materializer rejects unsealed input before creating output")) {
    return 1;
  }

  const auto refused_output = tree.path() / L"wrong-contract.capture";
  auto refused_request = attach_request(contract, executable, refused_output);
  refused_request.hook_table_fingerprint ^= 1;
  std::uint64_t refused_session = 0;
  if (!expect(
          api.attach(&refused_request, &refused_session) ==
                  GORE_AS_CAPTURE_BRIDGE_ABI_MISMATCH_V1 &&
              !std::filesystem::exists(refused_output),
          "wrong hook-table contract is refused before output creation")) {
    return 1;
  }
  const auto post_prepare_output = tree.path() / L"post-prepare.capture";
  auto post_prepare_request = attach_request(contract, executable, post_prepare_output);
  std::uint64_t post_prepare_session = 0;
  if (!expect(
          api.prepare_unload() == GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
              api.instrumentation_prepare_unload() ==
                  GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              api.attach(&post_prepare_request, &post_prepare_session) ==
                  GORE_AS_CAPTURE_BRIDGE_BUSY_V1 &&
              post_prepare_session == 0 && !std::filesystem::exists(post_prepare_output),
          "successful unload preparation permanently rejects a later attach")) {
    return 1;
  }
  return 0;
}

}  // namespace

int wmain(const int argc, wchar_t** argv) {
  if (argc != 4 || std::wstring_view(argv[1]) != L"--synthetic-e2e") {
    std::wcerr << L"usage: gore_as_capture_offline_host.exe --synthetic-e2e "
                  L"<gore_as_capture_bridge_fixture.dll> "
                  L"<gore_as_compiler_profile_capture_bridge.dll>\n";
    return 2;
  }
  return run_synthetic_e2e(
      std::filesystem::path(argv[2]), std::filesystem::path(argv[3]));
}
