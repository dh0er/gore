#include "path_safety.hpp"

#include <algorithm>
#include <array>
#include <cstring>
#include <cwchar>
#include <optional>
#include <vector>

namespace gore_as_capture::v1::detail {
namespace {

constexpr DWORD kOpenReparsePoint = FILE_FLAG_OPEN_REPARSE_POINT;

bool handle_is_regular_no_reparse(const HANDLE handle) noexcept {
  FILE_ATTRIBUTE_TAG_INFO attributes{};
  return GetFileInformationByHandleEx(
             handle, FileAttributeTagInfo, &attributes, sizeof(attributes)) != FALSE &&
         (attributes.FileAttributes &
          (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) == 0;
}

bool handle_is_directory_no_reparse(const HANDLE handle) noexcept {
  FILE_ATTRIBUTE_TAG_INFO attributes{};
  return GetFileInformationByHandleEx(
             handle, FileAttributeTagInfo, &attributes, sizeof(attributes)) != FALSE &&
         (attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 &&
         (attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) == 0;
}

bool same_file_identity(const HANDLE left, const HANDLE right) noexcept {
  FILE_ID_INFO left_id{};
  FILE_ID_INFO right_id{};
  return GetFileInformationByHandleEx(left, FileIdInfo, &left_id, sizeof(left_id)) != FALSE &&
         GetFileInformationByHandleEx(right, FileIdInfo, &right_id, sizeof(right_id)) != FALSE &&
         left_id.VolumeSerialNumber == right_id.VolumeSerialNumber &&
         std::memcmp(
             left_id.FileId.Identifier,
             right_id.FileId.Identifier,
             sizeof(left_id.FileId.Identifier)) == 0;
}

std::optional<std::filesystem::path> final_path(const HANDLE handle) {
  const DWORD required = GetFinalPathNameByHandleW(handle, nullptr, 0, 0);
  if (required == 0 || required >= 32768) {
    return std::nullopt;
  }
  std::vector<wchar_t> buffer(static_cast<std::size_t>(required) + 1u, L'\0');
  const DWORD length = GetFinalPathNameByHandleW(
      handle, buffer.data(), static_cast<DWORD>(buffer.size()), 0);
  if (length == 0 || static_cast<std::size_t>(length) >= buffer.size()) {
    return std::nullopt;
  }
  return std::filesystem::path(std::wstring_view(buffer.data(), length)).lexically_normal();
}

bool path_starts_with(
    const std::filesystem::path& candidate,
    const std::filesystem::path& prefix) noexcept {
  auto candidate_it = candidate.begin();
  for (auto prefix_it = prefix.begin(); prefix_it != prefix.end(); ++prefix_it, ++candidate_it) {
    if (candidate_it == candidate.end() ||
        _wcsicmp(candidate_it->c_str(), prefix_it->c_str()) != 0) {
      return false;
    }
  }
  return true;
}

bool has_named_stream(const std::filesystem::path& path) noexcept {
  const auto name = path.filename().native();
  return name.find(L':') != std::wstring::npos;
}

CaptureError reject_created_output(UniqueHandle& output) noexcept {
  FILE_DISPOSITION_INFO disposition{};
  disposition.DeleteFile = TRUE;
  const bool marked = SetFileInformationByHandle(
                          output.get(), FileDispositionInfo, &disposition, sizeof(disposition)) !=
                      FALSE;
  const bool closed = output.close();
  return marked && closed ? CaptureError::unsafe_output_path
                          : CaptureError::output_recovery_required;
}

}  // namespace

PinnedSourceHandles open_pinned_source(
    const std::filesystem::path& executable_path,
    const std::filesystem::path& loaded_module_path) noexcept {
  PinnedSourceHandles result;
  try {
    const auto executable = std::filesystem::absolute(executable_path).lexically_normal();
    const auto loaded = std::filesystem::absolute(loaded_module_path).lexically_normal();
    if (executable.empty() || loaded.empty() || has_named_stream(executable) ||
        has_named_stream(loaded)) {
      result.error = CaptureError::wrong_target;
      return result;
    }

    result.executable = UniqueHandle(CreateFileW(
        executable.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | kOpenReparsePoint, nullptr));
    if (!result.executable.valid()) {
      result.error = CaptureError::io_error;
      return result;
    }
    UniqueHandle loaded_handle(CreateFileW(
        loaded.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | kOpenReparsePoint, nullptr));
    if (!loaded_handle.valid() || !handle_is_regular_no_reparse(result.executable.get()) ||
        !handle_is_regular_no_reparse(loaded_handle.get()) ||
        !same_file_identity(result.executable.get(), loaded_handle.get())) {
      result.error = CaptureError::wrong_target;
      return result;
    }

    const auto executable_final = final_path(result.executable.get());
    if (!executable_final.has_value() || executable_final->parent_path().empty()) {
      result.error = CaptureError::wrong_target;
      return result;
    }
    result.executable_directory = UniqueHandle(CreateFileW(
        executable_final->parent_path().c_str(), FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | kOpenReparsePoint, nullptr));
    if (!result.executable_directory.valid() ||
        !handle_is_directory_no_reparse(result.executable_directory.get()) ||
        !final_path(result.executable_directory.get()).has_value()) {
      result.error = CaptureError::wrong_target;
      return result;
    }

    result.error = CaptureError::ok;
    return result;
  } catch (...) {
    result.error = CaptureError::invalid_argument;
    return result;
  }
}

PinnedOutputHandle create_pinned_output(
    const std::filesystem::path& output_path,
    const HANDLE executable_directory) noexcept {
  PinnedOutputHandle result;
  try {
    if (executable_directory == nullptr || executable_directory == INVALID_HANDLE_VALUE) {
      result.error = CaptureError::invalid_argument;
      return result;
    }
    const auto protected_tree = final_path(executable_directory);
    const auto output = std::filesystem::absolute(output_path).lexically_normal();
    if (!protected_tree.has_value() || output.empty() || has_named_stream(output)) {
      result.error = CaptureError::unsafe_output_path;
      return result;
    }

    result.output = UniqueHandle(CreateFileW(
        output.c_str(), GENERIC_READ | GENERIC_WRITE | DELETE, 0, nullptr, CREATE_NEW,
        FILE_ATTRIBUTE_NORMAL | kOpenReparsePoint, nullptr));
    if (!result.output.valid()) {
      const DWORD error = GetLastError();
      result.error = error == ERROR_FILE_EXISTS || error == ERROR_ALREADY_EXISTS
                         ? CaptureError::output_exists
                         : CaptureError::io_error;
      return result;
    }

    const auto output_final = final_path(result.output.get());
    if (!handle_is_regular_no_reparse(result.output.get()) || !output_final.has_value() ||
        has_named_stream(*output_final) || path_starts_with(*output_final, *protected_tree)) {
      result.error = reject_created_output(result.output);
      return result;
    }

    result.error = CaptureError::ok;
    return result;
  } catch (...) {
    if (result.output.valid()) {
      result.error = reject_created_output(result.output);
    } else {
      result.error = CaptureError::invalid_argument;
    }
    return result;
  }
}

}  // namespace gore_as_capture::v1::detail
