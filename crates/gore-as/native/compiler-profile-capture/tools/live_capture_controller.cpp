#include "gore_as_capture/live_bootstrap.h"

#include <bcrypt.h>
#include <tlhelp32.h>
#include <windows.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <limits>
#include <string>
#include <thread>
#include <vector>

namespace {

struct RemoteUnicodeString final {
  std::uint16_t length{};
  std::uint16_t maximum_length{};
  std::uint32_t padding{};
  std::uintptr_t buffer{};
};

struct RemoteLoaderContext final {
  std::uintptr_t ldr_load_dll{};
  std::uintptr_t bootstrap_offset{};
  std::uintptr_t control{};
  std::uintptr_t module{};
  std::int32_t ldr_status{std::numeric_limits<std::int32_t>::max()};
  std::uint32_t padding{};
  RemoteUnicodeString dll{};
  std::array<wchar_t, GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1> path{};
};

static_assert(offsetof(RemoteLoaderContext, ldr_load_dll) == 0x00);
static_assert(offsetof(RemoteLoaderContext, bootstrap_offset) == 0x08);
static_assert(offsetof(RemoteLoaderContext, control) == 0x10);
static_assert(offsetof(RemoteLoaderContext, module) == 0x18);
static_assert(offsetof(RemoteLoaderContext, ldr_status) == 0x20);
static_assert(offsetof(RemoteLoaderContext, dll) == 0x28);
static_assert(offsetof(RemoteLoaderContext, path) == 0x38);

// APC entry RCX=RemoteLoaderContext*. Call ntdll!LdrLoadDll, then invoke the bridge bootstrap at
// the already PE-audited export RVA. The routine preserves its only nonvolatile register and the
// Win64 shadow/alignment contract.
constexpr std::array<std::byte, 57> kEarlyLoaderApc{{
    std::byte{0x53},
    std::byte{0x48}, std::byte{0x83}, std::byte{0xec}, std::byte{0x20},
    std::byte{0x48}, std::byte{0x8b}, std::byte{0xd9},
    std::byte{0x33}, std::byte{0xc9},
    std::byte{0x33}, std::byte{0xd2},
    std::byte{0x4c}, std::byte{0x8d}, std::byte{0x43}, std::byte{0x28},
    std::byte{0x4c}, std::byte{0x8d}, std::byte{0x4b}, std::byte{0x18},
    std::byte{0x48}, std::byte{0x8b}, std::byte{0x03},
    std::byte{0xff}, std::byte{0xd0},
    std::byte{0x89}, std::byte{0x43}, std::byte{0x20},
    std::byte{0x85}, std::byte{0xc0},
    std::byte{0x78}, std::byte{0x13},
    std::byte{0x48}, std::byte{0x8b}, std::byte{0x43}, std::byte{0x18},
    std::byte{0x48}, std::byte{0x85}, std::byte{0xc0},
    std::byte{0x74}, std::byte{0x0a},
    std::byte{0x48}, std::byte{0x03}, std::byte{0x43}, std::byte{0x08},
    std::byte{0x48}, std::byte{0x8b}, std::byte{0x4b}, std::byte{0x10},
    std::byte{0xff}, std::byte{0xd0},
    std::byte{0x48}, std::byte{0x83}, std::byte{0xc4}, std::byte{0x20},
    std::byte{0x5b},
    std::byte{0xc3},
}};

constexpr std::uint64_t kShippingCacheBytes = 124'354'799;
constexpr std::uint64_t kBindsCacheBytes = 5'908'587;
constexpr std::array<std::uint8_t, 32> kShippingCacheSha256{{
    0xd0, 0xaf, 0xaf, 0x90, 0x9e, 0x62, 0x86, 0x7f,
    0xae, 0xdc, 0x36, 0x78, 0xa1, 0x17, 0x5f, 0x5e,
    0x8d, 0xe5, 0xe7, 0x84, 0xdc, 0x50, 0x3a, 0x14,
    0xff, 0xbd, 0xe4, 0x72, 0x6f, 0x29, 0x72, 0x31,
}};
constexpr std::array<std::uint8_t, 32> kBindsCacheSha256{{
    0x85, 0x4f, 0x58, 0xa6, 0x95, 0xd0, 0x17, 0x01,
    0x44, 0x95, 0x7f, 0x08, 0x5c, 0x1e, 0x8c, 0x0f,
    0x9e, 0xf4, 0x0b, 0x27, 0x1e, 0x35, 0xe9, 0x0f,
    0x79, 0xff, 0xbc, 0xcf, 0xf8, 0xd9, 0x99, 0xc5,
}};

class Handle final {
 public:
  Handle() noexcept = default;
  explicit Handle(const HANDLE value) noexcept : value_(value) {}
  ~Handle() {
    if (value_ != nullptr && value_ != INVALID_HANDLE_VALUE) (void)CloseHandle(value_);
  }
  Handle(const Handle&) = delete;
  Handle& operator=(const Handle&) = delete;
  Handle(Handle&& other) noexcept : value_(other.value_) { other.value_ = nullptr; }
  Handle& operator=(Handle&& other) noexcept {
    if (this == &other) return *this;
    if (value_ != nullptr && value_ != INVALID_HANDLE_VALUE) (void)CloseHandle(value_);
    value_ = other.value_;
    other.value_ = nullptr;
    return *this;
  }
  [[nodiscard]] HANDLE get() const noexcept { return value_; }
  [[nodiscard]] bool valid() const noexcept {
    return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
  }

 private:
  HANDLE value_{};
};

bool verify_file_seal(
    const std::filesystem::path& path,
    const std::uint64_t expected_bytes,
    const std::array<std::uint8_t, 32>& expected_sha256) {
  const Handle file(CreateFileW(
      path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
      FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT |
          FILE_FLAG_SEQUENTIAL_SCAN,
      nullptr));
  if (!file.valid() || GetFileType(file.get()) != FILE_TYPE_DISK) return false;
  FILE_ATTRIBUTE_TAG_INFO tag{};
  LARGE_INTEGER bytes{};
  if (GetFileInformationByHandleEx(
          file.get(), FileAttributeTagInfo, &tag, sizeof(tag)) == FALSE ||
      (tag.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0 ||
      GetFileSizeEx(file.get(), &bytes) == FALSE || bytes.QuadPart < 0 ||
      static_cast<std::uint64_t>(bytes.QuadPart) != expected_bytes) {
    return false;
  }

  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  std::vector<UCHAR> object;
  std::vector<UCHAR> buffer(1u << 20u);
  std::array<std::uint8_t, 32> digest{};
  bool ok = false;
  do {
    if (BCryptOpenAlgorithmProvider(
            &algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0) {
      break;
    }
    DWORD object_bytes = 0;
    DWORD returned = 0;
    if (BCryptGetProperty(
            algorithm, BCRYPT_OBJECT_LENGTH,
            reinterpret_cast<PUCHAR>(&object_bytes), sizeof(object_bytes),
            &returned, 0) < 0) {
      break;
    }
    object.resize(object_bytes);
    if (BCryptCreateHash(
            algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
      break;
    }
    std::uint64_t total = 0;
    for (;;) {
      DWORD read = 0;
      if (ReadFile(
              file.get(), buffer.data(), static_cast<DWORD>(buffer.size()),
              &read, nullptr) == FALSE) {
        break;
      }
      if (read == 0) {
        ok = total == expected_bytes &&
             BCryptFinishHash(hash, digest.data(),
                              static_cast<ULONG>(digest.size()), 0) >= 0 &&
             digest == expected_sha256;
        break;
      }
      if (total > expected_bytes - read ||
          BCryptHashData(hash, buffer.data(), read, 0) < 0) {
        break;
      }
      total += read;
    }
  } while (false);
  if (hash != nullptr) (void)BCryptDestroyHash(hash);
  if (algorithm != nullptr) (void)BCryptCloseAlgorithmProvider(algorithm, 0);
  return ok;
}

bool verify_target_inputs(const std::filesystem::path& executable) {
  auto script_root = executable.parent_path();
  for (int level = 0; level < 2; ++level) {
    if (!script_root.has_parent_path()) return false;
    script_root = script_root.parent_path();
  }
  script_root /= L"Script";
  std::error_code error;
  if (std::filesystem::exists(script_root / L"PrecompiledScript.Cache", error) ||
      error) {
    return false;
  }
  return verify_file_seal(
             script_root / L"PrecompiledScript_Shipping.Cache",
             kShippingCacheBytes, kShippingCacheSha256) &&
         verify_file_seal(
             script_root / L"Binds.Cache", kBindsCacheBytes,
             kBindsCacheSha256);
}

std::uintptr_t remote_module_base(
    const DWORD process_id,
    const std::wstring& filename) noexcept {
  const Handle snapshot(CreateToolhelp32Snapshot(
      TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, process_id));
  if (!snapshot.valid()) return 0;
  MODULEENTRY32W module{};
  module.dwSize = sizeof(module);
  if (Module32FirstW(snapshot.get(), &module) == FALSE) return 0;
  do {
    if (_wcsicmp(module.szModule, filename.c_str()) == 0) {
      return reinterpret_cast<std::uintptr_t>(module.modBaseAddr);
    }
    module.dwSize = sizeof(module);
  } while (Module32NextW(snapshot.get(), &module) != FALSE);
  return 0;
}

bool process_named_running(const std::wstring& filename) noexcept {
  const Handle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0));
  if (!snapshot.valid()) return false;
  PROCESSENTRY32W process{};
  process.dwSize = sizeof(process);
  if (Process32FirstW(snapshot.get(), &process) == FALSE) return false;
  do {
    if (_wcsicmp(process.szExeFile, filename.c_str()) == 0) return true;
    process.dwSize = sizeof(process);
  } while (Process32NextW(snapshot.get(), &process) != FALSE);
  return false;
}

std::uintptr_t remote_system_export(
    const HANDLE process,
    const DWORD process_id,
    const wchar_t* const module_name,
    const char* const name) noexcept {
  const auto local_module = GetModuleHandleW(module_name);
  const auto local_export = GetProcAddress(local_module, name);
  if (local_module == nullptr || local_export == nullptr) return 0;
  MEMORY_BASIC_INFORMATION local_region{};
  if (VirtualQuery(
          reinterpret_cast<const void*>(local_export),
          &local_region,
          sizeof(local_region)) != sizeof(local_region) ||
      local_region.AllocationBase == nullptr) {
    return 0;
  }
  const auto owner = static_cast<HMODULE>(local_region.AllocationBase);
  std::vector<wchar_t> path(32768);
  const auto chars = GetModuleFileNameW(owner, path.data(), static_cast<DWORD>(path.size()));
  if (chars == 0 || chars == path.size()) return 0;
  const auto filename = std::filesystem::path(
      std::wstring(path.data(), chars)).filename().wstring();
  const auto remote_owner = remote_module_base(process_id, filename);
  const auto local_owner = reinterpret_cast<std::uintptr_t>(owner);
  const auto local_address = reinterpret_cast<std::uintptr_t>(local_export);
  if (remote_owner != 0 && local_address >= local_owner) {
    return remote_owner + (local_address - local_owner);
  }

  // A just-created suspended process can have its KnownDLL mappings before Toolhelp publishes a
  // module list. Accept the per-boot shared mapping only after the allocation base and exact
  // exported instruction bytes independently match in the target.
  MEMORY_BASIC_INFORMATION remote_region{};
  std::array<std::byte, 16> local_bytes{};
  std::array<std::byte, 16> remote_bytes{};
  SIZE_T read = 0;
  std::copy_n(
      reinterpret_cast<const std::byte*>(local_export),
      local_bytes.size(),
      local_bytes.begin());
  if (VirtualQueryEx(
          process,
          reinterpret_cast<const void*>(local_address),
          &remote_region,
          sizeof(remote_region)) != sizeof(remote_region) ||
      remote_region.State != MEM_COMMIT ||
      remote_region.AllocationBase != reinterpret_cast<void*>(local_owner) ||
      ReadProcessMemory(
          process,
          reinterpret_cast<const void*>(local_address),
          remote_bytes.data(),
          remote_bytes.size(),
          &read) == FALSE ||
      read != remote_bytes.size() || remote_bytes != local_bytes) {
    return 0;
  }
  return local_address;
}

std::uintptr_t exported_offset(
    const std::filesystem::path& dll,
    const char* const name) noexcept {
  const auto module = LoadLibraryExW(
      dll.c_str(), nullptr, DONT_RESOLVE_DLL_REFERENCES);
  if (module == nullptr) return std::numeric_limits<std::uintptr_t>::max();
  const auto entry = GetProcAddress(module, name);
  const auto base = reinterpret_cast<std::uintptr_t>(module);
  const auto address = reinterpret_cast<std::uintptr_t>(entry);
  const auto offset = entry != nullptr && address >= base
                          ? address - base
                          : std::numeric_limits<std::uintptr_t>::max();
  (void)FreeLibrary(module);
  return offset;
}

bool inject_running_library(
    const HANDLE process,
    const DWORD process_id,
    const std::filesystem::path& library) noexcept {
  const auto path = library.native();
  const auto path_bytes = (path.size() + 1U) * sizeof(wchar_t);
  void* const remote_path = VirtualAllocEx(
      process, nullptr, path_bytes, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  SIZE_T written = 0;
  if (remote_path == nullptr ||
      WriteProcessMemory(
          process, remote_path, path.c_str(), path_bytes, &written) == FALSE ||
      written != path_bytes) {
    if (remote_path != nullptr) (void)VirtualFreeEx(process, remote_path, 0, MEM_RELEASE);
    return false;
  }
  std::uintptr_t load_library = 0;
  for (std::uint32_t attempt = 0; attempt < 500U && load_library == 0; ++attempt) {
    load_library = remote_system_export(
        process, process_id, L"kernel32.dll", "LoadLibraryW");
    if (load_library == 0) std::this_thread::sleep_for(std::chrono::milliseconds(10));
  }
  const Handle thread(load_library == 0 ? nullptr : CreateRemoteThread(
      process, nullptr, 0,
      reinterpret_cast<LPTHREAD_START_ROUTINE>(load_library), remote_path, 0, nullptr));
  if (!thread.valid()) {
    (void)VirtualFreeEx(process, remote_path, 0, MEM_RELEASE);
    return false;
  }
  const DWORD wait = WaitForSingleObject(thread.get(), 30'000);
  if (wait != WAIT_OBJECT_0) {
    // The remote thread may still be reading this allocation. The caller terminates and observes
    // the process before restoration; its address space then owns the cleanup.
    return false;
  }
  DWORD result = 0;
  const bool loaded = GetExitCodeThread(thread.get(), &result) != FALSE && result != 0;
  (void)VirtualFreeEx(process, remote_path, 0, MEM_RELEASE);
  return loaded;
}

bool diagnostics_ready(
    const HANDLE process,
    const std::filesystem::path& status_path) noexcept {
  const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(8);
  while (std::chrono::steady_clock::now() < deadline) {
    if (WaitForSingleObject(process, 0) == WAIT_OBJECT_0) return false;
    const Handle status(CreateFileW(
        status_path.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
        nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr));
    if (status.valid()) {
      std::array<char, 4096> bytes{};
      DWORD read = 0;
      if (ReadFile(status.get(), bytes.data(), static_cast<DWORD>(bytes.size() - 1U),
                   &read, nullptr) != FALSE) {
        const std::string_view value(bytes.data(), read);
        if (value == "ready\n" || value == "ready\r\n") return true;
        if (value.starts_with("unavailable:")) return false;
      }
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(20));
  }
  return false;
}

bool output_is_closed_and_nonempty(const std::filesystem::path& output) noexcept {
  const Handle file(CreateFileW(
      output.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
      FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
  if (!file.valid()) return false;
  LARGE_INTEGER bytes{};
  return GetFileSizeEx(file.get(), &bytes) != FALSE && bytes.QuadPart > 64;
}

BOOL CALLBACK close_process_window(const HWND window, const LPARAM parameter) {
  DWORD process_id = 0;
  (void)GetWindowThreadProcessId(window, &process_id);
  if (process_id == static_cast<DWORD>(parameter) && IsWindowVisible(window) != FALSE) {
    (void)PostMessageW(window, WM_CLOSE, 0, 0);
  }
  return TRUE;
}

bool read_control(
    const HANDLE process,
    const void* const remote,
    gore_as_capture_live_control_v1& control) noexcept {
  SIZE_T read = 0;
  return ReadProcessMemory(process, remote, &control, sizeof(control), &read) != FALSE &&
         read == sizeof(control);
}

std::uint32_t parse_timeout(const wchar_t* const text) noexcept {
  if (text == nullptr || *text == L'\0') return 0;
  wchar_t* end = nullptr;
  const auto value = wcstoul(text, &end, 10);
  return end != text && *end == L'\0' && value >= 30 && value <= 21'600
             ? static_cast<std::uint32_t>(value)
             : 0;
}

int attach_running_capture(
    const DWORD process_id,
    const std::filesystem::path& executable,
    const std::filesystem::path& bridge,
    const std::filesystem::path& output,
    const std::uint32_t timeout_seconds) {
  constexpr DWORD access = PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION |
                           PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE |
                           SYNCHRONIZE | PROCESS_TERMINATE;
  Handle process(OpenProcess(access, FALSE, process_id));
  if (!process.valid()) {
    std::wcerr << L"OpenProcess failed for Steam child " << process_id << L": "
               << GetLastError() << L'\n';
    return 4;
  }
  bool process_exited = false;
  const auto stop_process = [&] {
    if (!process_exited) {
      (void)TerminateProcess(process.get(), 0x474f5245u);
      (void)WaitForSingleObject(process.get(), 10'000);
      process_exited = true;
    }
  };

  std::uintptr_t load_library = 0;
  for (std::uint32_t attempt = 0; attempt < 500 && load_library == 0; ++attempt) {
    load_library = remote_system_export(
        process.get(), process_id, L"kernel32.dll", "LoadLibraryW");
    if (load_library == 0) std::this_thread::sleep_for(std::chrono::milliseconds(10));
  }
  const auto bootstrap_offset = exported_offset(bridge, "gore_as_capture_live_bootstrap_v1");
  if (load_library == 0 ||
      bootstrap_offset == std::numeric_limits<std::uintptr_t>::max()) {
    std::wcerr << L"cannot resolve running-child loader/bootstrap exports\n";
    stop_process();
    return 5;
  }

  const auto bridge_path = bridge.native();
  const auto bridge_path_bytes = (bridge_path.size() + 1) * sizeof(wchar_t);
  void* const remote_bridge_path = VirtualAllocEx(
      process.get(), nullptr, bridge_path_bytes, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  SIZE_T written = 0;
  if (remote_bridge_path == nullptr ||
      WriteProcessMemory(
          process.get(), remote_bridge_path, bridge_path.c_str(), bridge_path_bytes,
          &written) == FALSE ||
      written != bridge_path_bytes) {
    std::wcerr << L"cannot stage bridge in Steam child\n";
    stop_process();
    return 5;
  }
  const Handle loader_thread(CreateRemoteThread(
      process.get(), nullptr, 0,
      reinterpret_cast<LPTHREAD_START_ROUTINE>(load_library), remote_bridge_path, 0,
      nullptr));
  if (!loader_thread.valid() ||
      WaitForSingleObject(loader_thread.get(), 30'000) != WAIT_OBJECT_0) {
    std::wcerr << L"Steam-child bridge load did not complete\n";
    stop_process();
    return 5;
  }
  (void)VirtualFreeEx(process.get(), remote_bridge_path, 0, MEM_RELEASE);

  std::uintptr_t remote_bridge = 0;
  for (std::uint32_t attempt = 0; attempt < 500 && remote_bridge == 0; ++attempt) {
    remote_bridge = remote_module_base(process_id, bridge.filename().wstring());
    if (remote_bridge == 0) std::this_thread::sleep_for(std::chrono::milliseconds(10));
  }
  if (remote_bridge == 0) {
    std::wcerr << L"loaded bridge is absent from Steam child module list\n";
    stop_process();
    return 5;
  }

  gore_as_capture_live_control_v1 control{};
  control.struct_size = sizeof(control);
  control.magic = GORE_AS_CAPTURE_LIVE_CONTROL_MAGIC_V1;
  control.version = GORE_AS_CAPTURE_LIVE_CONTROL_VERSION_V1;
  control.observed_steam_build_id = 24539464;
  control.target_inputs_verified = 1;
  control.executable_path_chars = static_cast<std::uint32_t>(executable.native().size());
  control.output_path_chars = static_cast<std::uint32_t>(output.native().size());
  std::copy(executable.native().begin(), executable.native().end(), control.executable_path);
  std::copy(output.native().begin(), output.native().end(), control.output_path);
  if (BCryptGenRandom(
          nullptr, control.capture_id, static_cast<ULONG>(sizeof(control.capture_id)),
          BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0) {
    std::wcerr << L"capture-id generation failed\n";
    stop_process();
    return 5;
  }
  void* const remote_control = VirtualAllocEx(
      process.get(), nullptr, sizeof(control), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  if (remote_control == nullptr ||
      WriteProcessMemory(
          process.get(), remote_control, &control, sizeof(control), &written) == FALSE ||
      written != sizeof(control)) {
    std::wcerr << L"cannot stage Steam-child capture control\n";
    stop_process();
    return 5;
  }
  const Handle bootstrap_thread(CreateRemoteThread(
      process.get(), nullptr, 0,
      reinterpret_cast<LPTHREAD_START_ROUTINE>(remote_bridge + bootstrap_offset),
      remote_control, 0, nullptr));
  if (!bootstrap_thread.valid() ||
      WaitForSingleObject(bootstrap_thread.get(), 60'000) != WAIT_OBJECT_0 ||
      !read_control(process.get(), remote_control, control) ||
      control.status != GORE_AS_CAPTURE_LIVE_INSTALLED_V1) {
    std::wcerr << L"Steam-child capture bootstrap failed: state=" << control.status
               << L" bridge=" << control.bridge_status
               << L" image-validation=" << control.image_validation_status
               << L" patch-preflight=" << control.patch_preflight_detail
               << L" instrumentation=" << control.instrumentation_status << L'\n';
    stop_process();
    return 6;
  }
  std::wcout << L"capture installed in Steam child " << process_id
             << L"; timeout=" << timeout_seconds << L"s\n";

  const auto capture_deadline = std::chrono::steady_clock::now() +
                                std::chrono::seconds(timeout_seconds);
  auto next_progress =
      std::chrono::steady_clock::now() + std::chrono::seconds(60);
  LARGE_INTEGER performance_frequency{};
  (void)QueryPerformanceFrequency(&performance_frequency);
  const auto dispatch_seconds = [&]() {
    return performance_frequency.QuadPart > 0
               ? static_cast<long double>(control.dispatch_ticks) /
                     static_cast<long double>(performance_frequency.QuadPart)
               : 0.0L;
  };
  bool capture_ready = false;
  bool timed_out = true;
  DWORD exit_code = STILL_ACTIVE;
  while (std::chrono::steady_clock::now() < capture_deadline) {
    if (output_is_closed_and_nonempty(output) &&
        read_control(process.get(), remote_control, control) &&
        control.capture_outcome != GORE_AS_CAPTURE_LIVE_OUTCOME_PENDING_V1) {
      capture_ready = control.capture_outcome ==
                      GORE_AS_CAPTURE_LIVE_OUTCOME_SEALED_V1;
      timed_out = false;
      break;
    }
    if (WaitForSingleObject(process.get(), 0) == WAIT_OBJECT_0) {
      process_exited = true;
      timed_out = false;
      (void)GetExitCodeProcess(process.get(), &exit_code);
      break;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(250));
  }
  if (!capture_ready) {
    (void)read_control(process.get(), remote_control, control);
    std::wcerr << L"Steam-child capture did not seal: outcome=" << control.capture_outcome
               << L" failure-site=" << control.failure_site
               << L" failure-phase=" << control.failure_phase
               << L" failure-detail=0x" << std::hex << control.failure_detail
               << L" exit=0x" << exit_code << std::dec
               << L" previous-registration=" << control.previous_registration_site
               << L":" << control.previous_registration_result
               << L" last-registration=" << control.last_registration_site
               << L":" << control.last_registration_result
               << L" argument0='" << control.last_registration_argument0
               << L"' argument1='" << control.last_registration_argument1 << L"'"
               << L" scalars=0x" << std::hex << control.last_registration_scalar0
               << L",0x" << control.last_registration_scalar1
               << L",0x" << control.last_registration_scalar2 << std::dec
               << L" layout=" << control.last_object_alignment
               << L"/" << control.last_operations_alignment
               << L"/" << control.last_operations_available
               << L" type=" << control.last_reflected_type_id
               << L"/" << control.last_type_operations_kind
               << L"/" << control.last_type_value_size
               << L" timed-out=" << (timed_out ? 1 : 0) << L'\n';
    stop_process();
    return 7;
  }
  std::wcout << L"sealed Steam-child capture: " << output.c_str() << L'\n';
  if (!process_exited) {
    (void)EnumWindows(close_process_window, static_cast<LPARAM>(process_id));
    if (WaitForSingleObject(process.get(), 30'000) != WAIT_OBJECT_0) {
      stop_process();
    } else {
      process_exited = true;
    }
  }
  return 0;
}

}  // namespace

int wmain(const int argc, wchar_t** const argv) {
  const bool direct_mode = argc >= 2 &&
                           std::wstring_view(argv[1]) == L"--capture-windowed";
  const bool diagnostic_mode = argc >= 2 &&
                               std::wstring_view(argv[1]) ==
                                   L"--capture-windowed-with-diagnostics";
  const bool attach_mode = argc >= 2 &&
                           std::wstring_view(argv[1]) == L"--attach-running";
  if ((!direct_mode && !diagnostic_mode && !attach_mode) ||
      (direct_mode && argc != 5 && argc != 6) ||
      (diagnostic_mode && argc != 8 && argc != 9) ||
      (attach_mode && argc != 6 && argc != 7)) {
    std::wcerr << L"usage: gore_as_capture_live_controller.exe --capture-windowed "
                  L"<G1R-Win64-Shipping.exe> <production-bridge.dll> <new.capture> "
                  L"[timeout-seconds]\n"
                  L"   or: gore_as_capture_live_controller.exe "
                  L"--capture-windowed-with-diagnostics <G1R-Win64-Shipping.exe> "
                  L"<production-bridge.dll> <diagnostics.dll> <diagnostics.txt> "
                  L"<diagnostics-status.txt> <new.capture> [timeout-seconds]\n"
                  L"   or: gore_as_capture_live_controller.exe --attach-running <pid> "
                  L"<G1R-Win64-Shipping.exe> <production-bridge.dll> <new.capture> "
                  L"[timeout-seconds]\n";
    return 2;
  }
  const auto executable_argument = attach_mode ? argv[3] : argv[2];
  const auto bridge_argument = attach_mode ? argv[4] : argv[3];
  const auto output_argument = attach_mode ? argv[5] : (diagnostic_mode ? argv[7] : argv[4]);
  const auto timeout_seconds =
      attach_mode ? (argc == 7 ? parse_timeout(argv[6]) : 600u)
                  : (diagnostic_mode
                         ? (argc == 9 ? parse_timeout(argv[8]) : 600u)
                         : (argc == 6 ? parse_timeout(argv[5]) : 600u));
  DWORD attached_process_id = 0;
  if (attach_mode) {
    wchar_t* end = nullptr;
    const auto parsed = wcstoul(argv[2], &end, 10);
    if (end == argv[2] || *end != L'\0' || parsed == 0 ||
        parsed > std::numeric_limits<DWORD>::max()) {
      std::wcerr << L"invalid running process id\n";
      return 2;
    }
    attached_process_id = static_cast<DWORD>(parsed);
  }
  if (timeout_seconds == 0) {
    std::wcerr << L"timeout must be between 30 and 21600 seconds\n";
    return 2;
  }

  std::error_code error;
  const auto executable = std::filesystem::weakly_canonical(executable_argument, error);
  if (error || !std::filesystem::is_regular_file(executable, error)) {
    std::wcerr << L"target executable is missing\n";
    return 2;
  }
  const auto bridge = std::filesystem::weakly_canonical(bridge_argument, error);
  if (error || !std::filesystem::is_regular_file(bridge, error)) {
    std::wcerr << L"production bridge is missing\n";
    return 2;
  }
  std::filesystem::path diagnostics;
  std::filesystem::path diagnostics_output;
  std::filesystem::path diagnostics_status;
  if (diagnostic_mode) {
    diagnostics = std::filesystem::weakly_canonical(argv[4], error);
    if (error || !std::filesystem::is_regular_file(diagnostics, error)) {
      std::wcerr << L"diagnostics helper is missing\n";
      return 2;
    }
    diagnostics_output = std::filesystem::absolute(argv[5], error).lexically_normal();
    diagnostics_status = std::filesystem::absolute(argv[6], error).lexically_normal();
    if (error || diagnostics_output.empty() || diagnostics_status.empty() ||
        std::filesystem::exists(diagnostics_output, error) || error ||
        std::filesystem::exists(diagnostics_status, error) || error ||
        !std::filesystem::is_directory(diagnostics_output.parent_path(), error) || error ||
        !std::filesystem::is_directory(diagnostics_status.parent_path(), error) || error) {
      std::wcerr << L"diagnostics outputs must be new files in existing directories\n";
      return 2;
    }
  }
  const auto output = std::filesystem::absolute(output_argument, error).lexically_normal();
  if (error || output.empty() || std::filesystem::exists(output, error) ||
      !std::filesystem::is_directory(output.parent_path(), error)) {
    std::wcerr << L"output must be a new file in an existing directory\n";
    return 2;
  }
  if (executable.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 ||
      bridge.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 ||
      output.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 ||
      (diagnostic_mode &&
       (diagnostics.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 ||
        diagnostics_output.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1 ||
        diagnostics_status.native().size() >= GORE_AS_CAPTURE_LIVE_PATH_CHARS_V1))) {
    std::wcerr << L"path exceeds the live-control ABI bound\n";
    return 2;
  }
  bool target_inputs_verified = false;
  try {
    target_inputs_verified = verify_target_inputs(executable);
  } catch (...) {
    target_inputs_verified = false;
  }
  if (!target_inputs_verified) {
    std::wcerr << L"Shipping/Binds input seal mismatch or development cache present\n";
    return 3;
  }
  if ((direct_mode || diagnostic_mode) &&
      process_named_running(executable.filename().wstring())) {
    std::wcerr << L"target process is already running\n";
    return 3;
  }
  if (attach_mode) {
    return attach_running_capture(
        attached_process_id, executable, bridge, output, timeout_seconds);
  }

  // The pinned game normally delegates a direct launch back to Steam.  Supplying the
  // exact AppID in the inherited environment keeps this suspended, already-authorized
  // Steam launch in the process we instrumented before its first instruction.  The
  // generator switches are part of the capture contract: a cache-only menu launch does
  // not execute InitialCompile and therefore cannot reach the frontend seal boundary.
  if (SetEnvironmentVariableW(L"SteamAppId", L"1297900") == FALSE ||
      SetEnvironmentVariableW(L"SteamGameId", L"1297900") == FALSE) {
    std::wcerr << L"cannot establish pinned Steam AppID environment\n";
    return 4;
  }
  if (diagnostic_mode &&
      (SetEnvironmentVariableW(L"GORE_AS_ERRFILE", diagnostics_output.c_str()) == FALSE ||
       SetEnvironmentVariableW(L"GORE_AS_STATUSFILE", diagnostics_status.c_str()) == FALSE)) {
    std::wcerr << L"cannot establish diagnostics output environment\n";
    return 4;
  }
  auto command =
      L"\"" + executable.native() +
      L"\" -as-development-mode -as-generate-precompiled-data"
      L" -as-skip-threaded-initialize -as-exit-on-error"
      L" -windowed -ResX=1280 -ResY=720";
  std::vector<wchar_t> command_buffer(command.begin(), command.end());
  command_buffer.push_back(L'\0');
  auto working_directory = executable.parent_path();
  for (int level = 0; level < 3 && working_directory.has_parent_path(); ++level) {
    working_directory = working_directory.parent_path();
  }
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION information{};
  if (CreateProcessW(
          executable.c_str(), command_buffer.data(), nullptr, nullptr, FALSE,
          CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP, nullptr,
          working_directory.c_str(), &startup, &information) == FALSE) {
    std::wcerr << L"CreateProcessW failed: " << GetLastError() << L'\n';
    return 4;
  }
  Handle process(information.hProcess);
  Handle primary_thread(information.hThread);
  bool process_exited = false;
  const auto stop_process = [&] {
    if (!process_exited) {
      (void)TerminateProcess(process.get(), 0x474f5245u);
      (void)WaitForSingleObject(process.get(), 10'000);
      process_exited = true;
    }
  };
  Handle process_job(CreateJobObjectW(nullptr, nullptr));
  JOBOBJECT_EXTENDED_LIMIT_INFORMATION job_information{};
  job_information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
  if (!process_job.valid() ||
      SetInformationJobObject(
          process_job.get(), JobObjectExtendedLimitInformation,
          &job_information, sizeof(job_information)) == FALSE ||
      AssignProcessToJobObject(process_job.get(), process.get()) == FALSE) {
    std::wcerr << L"cannot bind captured game to the controller lifetime\n";
    stop_process();
    return 4;
  }

  const auto ldr_load_dll = remote_system_export(
      process.get(), information.dwProcessId, L"ntdll.dll", "LdrLoadDll");
  const auto bootstrap_offset = exported_offset(bridge, "gore_as_capture_live_bootstrap_v1");
  if (ldr_load_dll == 0 ||
      bootstrap_offset == std::numeric_limits<std::uintptr_t>::max()) {
    std::wcerr << L"cannot resolve exact early bootstrap exports: LdrLoadDll=0x"
               << std::hex << ldr_load_dll << L" bootstrap-offset=0x"
               << bootstrap_offset << std::dec << L'\n';
    stop_process();
    return 5;
  }

  gore_as_capture_live_control_v1 control{};
  control.struct_size = sizeof(control);
  control.magic = GORE_AS_CAPTURE_LIVE_CONTROL_MAGIC_V1;
  control.version = GORE_AS_CAPTURE_LIVE_CONTROL_VERSION_V1;
  control.observed_steam_build_id = 24539464;
  control.target_inputs_verified = 1;
  control.executable_path_chars = static_cast<std::uint32_t>(executable.native().size());
  control.output_path_chars = static_cast<std::uint32_t>(output.native().size());
  std::copy(executable.native().begin(), executable.native().end(), control.executable_path);
  std::copy(output.native().begin(), output.native().end(), control.output_path);
  if (BCryptGenRandom(
          nullptr, control.capture_id, static_cast<ULONG>(sizeof(control.capture_id)),
          BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0) {
    std::wcerr << L"capture-id generation failed\n";
    stop_process();
    return 5;
  }
  void* const remote_control = VirtualAllocEx(
      process.get(), nullptr, sizeof(control), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  SIZE_T written = 0;
  if (remote_control == nullptr ||
      WriteProcessMemory(
          process.get(), remote_control, &control, sizeof(control), &written) == FALSE ||
      written != sizeof(control)) {
    std::wcerr << L"cannot stage exact capture control\n";
    stop_process();
    return 5;
  }

  const auto bridge_path = bridge.native();
  RemoteLoaderContext loader{};
  loader.ldr_load_dll = ldr_load_dll;
  loader.bootstrap_offset = bootstrap_offset;
  loader.control = reinterpret_cast<std::uintptr_t>(remote_control);
  loader.dll.length = static_cast<std::uint16_t>(bridge_path.size() * sizeof(wchar_t));
  loader.dll.maximum_length =
      static_cast<std::uint16_t>((bridge_path.size() + 1) * sizeof(wchar_t));
  std::copy(bridge_path.begin(), bridge_path.end(), loader.path.begin());
  void* const remote_loader = VirtualAllocEx(
      process.get(), nullptr, sizeof(loader), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  if (remote_loader == nullptr) {
    std::wcerr << L"cannot allocate exact early-loader context\n";
    stop_process();
    return 5;
  }
  loader.dll.buffer = reinterpret_cast<std::uintptr_t>(remote_loader) +
                      offsetof(RemoteLoaderContext, path);
  if (WriteProcessMemory(
          process.get(), remote_loader, &loader, sizeof(loader), &written) == FALSE ||
      written != sizeof(loader)) {
    std::wcerr << L"cannot stage exact early-loader context\n";
    stop_process();
    return 5;
  }
  void* const remote_apc = VirtualAllocEx(
      process.get(), nullptr, kEarlyLoaderApc.size(), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  DWORD old_protection = 0;
  if (remote_apc == nullptr ||
      WriteProcessMemory(
          process.get(), remote_apc, kEarlyLoaderApc.data(), kEarlyLoaderApc.size(),
          &written) == FALSE ||
      written != kEarlyLoaderApc.size() ||
      VirtualProtectEx(
          process.get(), remote_apc, kEarlyLoaderApc.size(), PAGE_EXECUTE_READ,
          &old_protection) == FALSE ||
      FlushInstructionCache(process.get(), remote_apc, kEarlyLoaderApc.size()) == FALSE) {
    std::wcerr << L"cannot stage executable early-loader APC\n";
    stop_process();
    return 5;
  }
  if (QueueUserAPC(
          reinterpret_cast<PAPCFUNC>(remote_apc),
          primary_thread.get(),
          reinterpret_cast<ULONG_PTR>(remote_loader)) == 0 ||
      ResumeThread(primary_thread.get()) == std::numeric_limits<DWORD>::max()) {
    std::wcerr << L"cannot schedule main-thread capture bootstrap\n";
    stop_process();
    return 5;
  }

  const auto bootstrap_deadline =
      std::chrono::steady_clock::now() + std::chrono::seconds(60);
  while (std::chrono::steady_clock::now() < bootstrap_deadline) {
    if (WaitForSingleObject(process.get(), 0) == WAIT_OBJECT_0) {
      process_exited = true;
      break;
    }
    if (!read_control(process.get(), remote_control, control)) break;
    if (control.status == GORE_AS_CAPTURE_LIVE_INSTALLED_V1 ||
        control.status == GORE_AS_CAPTURE_LIVE_FAILED_V1) {
      break;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(25));
  }
  if (control.status != GORE_AS_CAPTURE_LIVE_INSTALLED_V1) {
    RemoteLoaderContext observed_loader{};
    SIZE_T read = 0;
    (void)ReadProcessMemory(
        process.get(), remote_loader, &observed_loader, sizeof(observed_loader), &read);
    std::wcerr << L"capture bootstrap failed: state=" << control.status
               << L" bridge=" << control.bridge_status
               << L" image-validation=" << control.image_validation_status
               << L" patch-preflight=" << control.patch_preflight_detail
               << L" source-unwind-mask=0x" << std::hex << control.source_unwind_mask
               << std::dec
               << L" instrumentation=" << control.instrumentation_status
               << L" ldr-status=0x" << std::hex
               << static_cast<std::uint32_t>(observed_loader.ldr_status)
               << L" module=0x" << observed_loader.module << std::dec << L'\n';
    stop_process();
    return 6;
  }
  if (diagnostic_mode) {
    const auto injection_deadline =
        std::chrono::steady_clock::now() + std::chrono::seconds(2);
    while (std::chrono::steady_clock::now() < injection_deadline) {
      if (WaitForSingleObject(process.get(), 0) == WAIT_OBJECT_0) {
        process_exited = true;
        break;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }
    if (process_exited ||
        !inject_running_library(
            process.get(), information.dwProcessId, diagnostics) ||
        !diagnostics_ready(process.get(), diagnostics_status)) {
      std::wcerr << L"same-run diagnostics injection did not reach ready state\n";
      stop_process();
      return 6;
    }
  }
  std::wcout << L"capture instrumentation installed; game resumed windowed; timeout="
             << timeout_seconds << L"s\n";

  const auto capture_deadline = std::chrono::steady_clock::now() +
                                std::chrono::seconds(timeout_seconds);
  auto next_progress =
      std::chrono::steady_clock::now() + std::chrono::seconds(60);
  LARGE_INTEGER performance_frequency{};
  (void)QueryPerformanceFrequency(&performance_frequency);
  const auto dispatch_seconds = [&]() {
    return performance_frequency.QuadPart > 0
               ? static_cast<long double>(control.dispatch_ticks) /
                     static_cast<long double>(performance_frequency.QuadPart)
               : 0.0L;
  };
  constexpr std::array<std::uint32_t, GORE_AS_CAPTURE_LIVE_STAGE_BUCKETS_V1>
      observer_stages{
          0x100u, 0x101u, 0x102u, 0x103u, 0x200u, 0x300u,
          0x301u, 0x302u, 0x303u, 0x304u, 0x305u, 0x306u,
          0x307u, 0x308u, 0x400u, 0x401u, 0x402u, 0x403u,
          0x500u, 0x501u};
  const auto hottest_stage = [&]() {
    std::size_t hottest = 0;
    for (std::size_t index = 1; index < observer_stages.size(); ++index) {
      if (control.observer_stage_ticks[index] >
          control.observer_stage_ticks[hottest]) {
        hottest = index;
      }
    }
    return hottest;
  };
  bool capture_ready = false;
  bool capture_terminal = false;
  bool capture_timed_out = true;
  DWORD process_exit_code = STILL_ACTIVE;
  while (std::chrono::steady_clock::now() < capture_deadline) {
    const auto now = std::chrono::steady_clock::now();
    if (now >= next_progress &&
        read_control(process.get(), remote_control, control)) {
      const auto hot = hottest_stage();
      const auto hot_seconds = performance_frequency.QuadPart > 0
                                   ? static_cast<long double>(
                                         control.observer_stage_ticks[hot]) /
                                         static_cast<long double>(
                                             performance_frequency.QuadPart)
                                   : 0.0L;
      std::wcout << L"capture progress: registrations="
                 << control.registration_count
                 << L" dispatches=" << control.dispatch_calls
                 << L" observer-seconds=" << static_cast<double>(dispatch_seconds())
                 << L" last-site=" << control.last_registration_site
                 << L" observer-stage=0x" << std::hex << control.observer_stage
                 << L" hottest-stage=0x" << observer_stages[hot] << std::dec
                 << L" hottest-seconds=" << static_cast<double>(hot_seconds)
                 << L'\n'
                 << std::flush;
      next_progress = now + std::chrono::seconds(60);
    }
    if (output_is_closed_and_nonempty(output)) {
      if (read_control(process.get(), remote_control, control)) {
        capture_ready = control.capture_outcome ==
                        GORE_AS_CAPTURE_LIVE_OUTCOME_SEALED_V1;
        capture_terminal = control.capture_outcome !=
                           GORE_AS_CAPTURE_LIVE_OUTCOME_PENDING_V1;
        if (capture_terminal) {
          capture_timed_out = false;
          break;
        }
      }
    }
    if (WaitForSingleObject(process.get(), 0) == WAIT_OBJECT_0) {
      process_exited = true;
      capture_timed_out = false;
      (void)GetExitCodeProcess(process.get(), &process_exit_code);
      capture_ready = output_is_closed_and_nonempty(output) &&
                      read_control(process.get(), remote_control, control) &&
                      control.capture_outcome ==
                          GORE_AS_CAPTURE_LIVE_OUTCOME_SEALED_V1;
      break;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(250));
  }
  if (!capture_ready) {
    (void)read_control(process.get(), remote_control, control);
    std::wcerr << L"capture did not seal: outcome=" << control.capture_outcome
               << L" failure-site=" << control.failure_site
               << L" failure-phase=" << control.failure_phase
               << L" failure-detail=0x" << std::hex << control.failure_detail << std::dec
               << L" owner-thread=" << control.capture_owner_thread
               << L" failure-thread=" << control.failure_thread
               << L" previous-registration=" << control.previous_registration_site
               << L":" << control.previous_registration_result
               << L" last-registration=" << control.last_registration_site
               << L":" << control.last_registration_result
               << L" argument0='" << control.last_registration_argument0
               << L"' argument1='" << control.last_registration_argument1 << L"'"
               << L" scalars=0x" << std::hex << control.last_registration_scalar0
               << L",0x" << control.last_registration_scalar1
               << L",0x" << control.last_registration_scalar2 << std::dec
               << L" layout=" << control.last_object_alignment
               << L"/" << control.last_operations_alignment
               << L"/" << control.last_operations_available
               << L" type=" << control.last_reflected_type_id
               << L"/" << control.last_type_operations_kind
               << L"/" << control.last_type_value_size
               << L" projected-counts=";
    for (std::size_t index = 0; index < 8; ++index) {
      if (index != 0) std::wcerr << L",";
      std::wcerr << control.projected_registry_counts[index];
    }
    std::wcerr << L" reflected-counts=";
    for (std::size_t index = 0; index < 8; ++index) {
      if (index != 0) std::wcerr << L",";
      std::wcerr << control.reflected_registry_counts[index];
    }
    std::wcerr << L" container-header=";
    for (std::size_t index = 0; index < 8; ++index) {
      if (index != 0) std::wcerr << L",";
      std::wcerr << L"0x" << std::hex << control.last_container_header[index]
                 << std::dec;
    }
    std::wcerr
               << L" registrations=" << control.registration_count
               << L" dispatches=" << control.dispatch_calls
               << L" observer-seconds=" << static_cast<double>(dispatch_seconds())
               << L" observer-stage=0x" << std::hex << control.observer_stage << std::dec
               << L" timed-out=" << (capture_timed_out ? 1 : 0)
               << L" process-exit=0x" << std::hex << process_exit_code << std::dec << L'\n';
    stop_process();
    return 7;
  }
  std::wcout << L"sealed capture is closed and readable: " << output.c_str() << L'\n';
  std::wcout << L"capture metrics: registrations=" << control.registration_count
             << L" dispatches=" << control.dispatch_calls
             << L" observer-seconds=" << static_cast<double>(dispatch_seconds())
             << L'\n';

  if (!process_exited) {
    (void)EnumWindows(close_process_window, static_cast<LPARAM>(information.dwProcessId));
    if (WaitForSingleObject(process.get(), 30'000) != WAIT_OBJECT_0) {
      std::wcout << L"window close timed out; terminating the dedicated capture process\n";
      stop_process();
    } else {
      process_exited = true;
    }
  }
  return 0;
}
