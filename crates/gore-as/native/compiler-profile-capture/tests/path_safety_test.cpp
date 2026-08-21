#include "path_safety.hpp"

#include <windows.h>
#include <winioctl.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <string>

namespace {

using gore_as_capture::v1::CaptureError;
using gore_as_capture::v1::detail::PinnedOutputHandle;
using gore_as_capture::v1::detail::PinnedSourceHandles;

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
              (L"gore-as-capture-path-test-" + std::to_wstring(GetCurrentProcessId()) + L"-" +
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
    if (!junction_.empty()) {
      RemoveDirectoryW(junction_.c_str());
    }
    if (valid_) {
      std::error_code ignored;
      std::filesystem::remove_all(path_, ignored);
    }
  }

  [[nodiscard]] bool valid() const noexcept { return valid_; }
  [[nodiscard]] const std::filesystem::path& path() const noexcept { return path_; }
  void remember_junction(std::filesystem::path junction) { junction_ = std::move(junction); }

 private:
  std::filesystem::path path_;
  std::filesystem::path junction_;
  bool valid_{};
};

bool write_new(const std::filesystem::path& path, const std::string_view bytes) {
  const HANDLE file = CreateFileW(
      path.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
  if (file == INVALID_HANDLE_VALUE) {
    return false;
  }
  DWORD written = 0;
  const bool ok = WriteFile(
                      file, bytes.data(), static_cast<DWORD>(bytes.size()), &written, nullptr) !=
                  FALSE &&
                  written == static_cast<DWORD>(bytes.size()) && FlushFileBuffers(file) != FALSE;
  CloseHandle(file);
  return ok;
}

#pragma pack(push, 1)
struct MountPointBuffer final {
  DWORD tag{};
  WORD data_length{};
  WORD reserved{};
  WORD substitute_offset{};
  WORD substitute_length{};
  WORD print_offset{};
  WORD print_length{};
  std::array<wchar_t, 16384> path{};
};
#pragma pack(pop)

bool create_junction(
    const std::filesystem::path& junction,
    const std::filesystem::path& target) {
  if (CreateDirectoryW(junction.c_str(), nullptr) == FALSE) {
    return false;
  }
  const HANDLE directory = CreateFileW(
      junction.c_str(), GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
      FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
  if (directory == INVALID_HANDLE_VALUE) {
    RemoveDirectoryW(junction.c_str());
    return false;
  }

  const std::wstring print = std::filesystem::absolute(target).lexically_normal().native();
  const std::wstring substitute = L"\\??\\" + print;
  MountPointBuffer buffer{};
  buffer.tag = IO_REPARSE_TAG_MOUNT_POINT;
  buffer.substitute_length = static_cast<WORD>(substitute.size() * sizeof(wchar_t));
  buffer.print_offset = static_cast<WORD>(buffer.substitute_length + sizeof(wchar_t));
  buffer.print_length = static_cast<WORD>(print.size() * sizeof(wchar_t));
  std::memcpy(buffer.path.data(), substitute.data(), buffer.substitute_length);
  std::memcpy(
      reinterpret_cast<std::byte*>(buffer.path.data()) + buffer.print_offset,
      print.data(),
      buffer.print_length);
  const std::size_t path_bytes = static_cast<std::size_t>(buffer.print_offset) +
                                 buffer.print_length + sizeof(wchar_t);
  buffer.data_length = static_cast<WORD>(8u + path_bytes);
  const DWORD input_bytes = static_cast<DWORD>(offsetof(MountPointBuffer, path) + path_bytes);
  DWORD returned = 0;
  const bool ok = DeviceIoControl(
                      directory, FSCTL_SET_REPARSE_POINT, &buffer, input_bytes, nullptr, 0,
                      &returned, nullptr) != FALSE;
  CloseHandle(directory);
  if (!ok) {
    RemoveDirectoryW(junction.c_str());
  }
  return ok;
}

bool expect(const bool condition, const char* message) {
  if (!condition) {
    std::cerr << "FAILED: " << message << '\n';
  }
  return condition;
}

}  // namespace

int wmain() {
  TempTree tree;
  if (!expect(tree.valid(), "allocate temp tree")) {
    return 1;
  }
  const auto executable_directory = tree.path() / L"game";
  const auto external_directory = tree.path() / L"external";
  std::filesystem::create_directories(executable_directory);
  std::filesystem::create_directories(external_directory);
  const auto executable = executable_directory / L"G1R-Win64-Shipping.exe";
  if (!expect(write_new(executable, "synthetic executable"), "create executable fixture")) {
    return 1;
  }

  PinnedSourceHandles source =
      gore_as_capture::v1::detail::open_pinned_source(executable, executable);
  if (!expect(source.error == CaptureError::ok, "pin exact source identity")) {
    return 1;
  }

  const auto source_link = external_directory / L"source-link.exe";
  if (CreateSymbolicLinkW(
          source_link.c_str(), executable.c_str(),
          SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE) != FALSE) {
    PinnedSourceHandles linked_source =
        gore_as_capture::v1::detail::open_pinned_source(source_link, executable);
    if (!expect(linked_source.error != CaptureError::ok,
                "reject a final-component source reparse point")) {
      return 1;
    }
  } else {
    std::cout << "SKIP: unprivileged file symlink creation unavailable on this host\n";
  }

  const auto safe_path = external_directory / L"safe.capture";
  PinnedOutputHandle safe = gore_as_capture::v1::detail::create_pinned_output(
      safe_path, source.executable_directory.get());
  if (!expect(safe.error == CaptureError::ok && std::filesystem::exists(safe_path),
              "create external output")) {
    return 1;
  }
  (void)safe.output.close();

  const auto direct_unsafe = executable_directory / L"direct.capture";
  PinnedOutputHandle direct = gore_as_capture::v1::detail::create_pinned_output(
      direct_unsafe, source.executable_directory.get());
  if (!expect(direct.error == CaptureError::unsafe_output_path &&
                  !std::filesystem::exists(direct_unsafe),
              "delete direct in-tree output by handle")) {
    return 1;
  }

  const auto existing = external_directory / L"existing.capture";
  if (!expect(write_new(existing, "keep"), "create existing output fixture")) {
    return 1;
  }
  PinnedOutputHandle collision = gore_as_capture::v1::detail::create_pinned_output(
      existing, source.executable_directory.get());
  if (!expect(collision.error == CaptureError::output_exists &&
                  std::filesystem::file_size(existing) == 4,
              "CREATE_NEW preserves existing output")) {
    return 1;
  }

  const auto junction = external_directory / L"apparently-external";
  if (create_junction(junction, executable_directory)) {
    tree.remember_junction(junction);
    const auto redirected = junction / L"redirected.capture";
    PinnedOutputHandle through_junction = gore_as_capture::v1::detail::create_pinned_output(
        redirected, source.executable_directory.get());
    if (!expect(through_junction.error == CaptureError::unsafe_output_path &&
                    !std::filesystem::exists(executable_directory / L"redirected.capture"),
                "resolve junction target and delete only created output")) {
      return 1;
    }
  } else {
    std::cout << "SKIP: junction creation unavailable on this host\n";
  }

  return 0;
}
