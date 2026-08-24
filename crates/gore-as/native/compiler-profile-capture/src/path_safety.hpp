#pragma once

#include "gore_as_capture/session.hpp"

#include <windows.h>

#include <filesystem>

namespace gore_as_capture::v1::detail {

class UniqueHandle final {
 public:
  UniqueHandle() noexcept = default;
  explicit UniqueHandle(HANDLE handle) noexcept : handle_(handle) {}
  ~UniqueHandle() { (void)close(); }

  UniqueHandle(UniqueHandle&& other) noexcept : handle_(other.release()) {}
  UniqueHandle& operator=(UniqueHandle&& other) noexcept {
    if (this != &other) {
      (void)close();
      handle_ = other.release();
    }
    return *this;
  }

  UniqueHandle(const UniqueHandle&) = delete;
  UniqueHandle& operator=(const UniqueHandle&) = delete;

  [[nodiscard]] HANDLE get() const noexcept { return handle_; }
  [[nodiscard]] bool valid() const noexcept {
    return handle_ != nullptr && handle_ != INVALID_HANDLE_VALUE;
  }
  [[nodiscard]] HANDLE release() noexcept {
    const HANDLE result = handle_;
    handle_ = INVALID_HANDLE_VALUE;
    return result;
  }
  [[nodiscard]] bool close() noexcept {
    if (!valid()) {
      handle_ = INVALID_HANDLE_VALUE;
      return true;
    }
    const HANDLE handle = release();
    return CloseHandle(handle) != FALSE;
  }

 private:
  HANDLE handle_{INVALID_HANDLE_VALUE};
};

struct PinnedSourceHandles final {
  CaptureError error{CaptureError::invalid_state};
  UniqueHandle executable;
  /// Held without delete sharing for the complete capture session.
  UniqueHandle executable_directory;
};

struct PinnedOutputHandle final {
  CaptureError error{CaptureError::invalid_state};
  UniqueHandle output;
};

/// Open the caller spelling and the loaded-module spelling without following a final reparse
/// point, require both handles to identify the same regular file, and retain the resolved
/// executable directory against rename/delete for the capture lifetime.
[[nodiscard]] PinnedSourceHandles open_pinned_source(
    const std::filesystem::path& executable_path,
    const std::filesystem::path& loaded_module_path) noexcept;

/// Create exactly one new regular output and compare its handle-resolved path with the held,
/// handle-resolved executable directory. Any unverifiable or in-tree output is marked for deletion
/// by its own handle before that handle is closed. Failure to do so is recovery-required.
[[nodiscard]] PinnedOutputHandle create_pinned_output(
    const std::filesystem::path& output_path,
    HANDLE executable_directory) noexcept;

}  // namespace gore_as_capture::v1::detail
