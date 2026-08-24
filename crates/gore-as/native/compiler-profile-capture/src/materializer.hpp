#pragma once

#include "gore_as_capture/format.hpp"

#include <cstdint>
#include <filesystem>

namespace gore_as_capture::v1::offline {

enum class MaterializeError : std::uint32_t {
  ok = 0,
  invalid_argument,
  input_io,
  input_reparse,
  input_too_large,
  malformed_capture,
  target_mismatch,
  digest_mismatch,
  incomplete_capture,
  output_exists,
  output_unsafe,
  output_io,
  output_recovery_required,
  crypto_error,
};

struct MaterializeResult final {
  MaterializeError error{MaterializeError::invalid_argument};
  std::uint64_t record_count{};
  Digest sealed_stream_sha256{};
};

/// Validate and materialize a pointer-neutral wire summary from one sealed capture.
///
/// This deliberately does not claim to create a qualified compiler profile. The existing Rust
/// decoder owns typed registry/frontend validation and profile projection. Input is opened without
/// following a final reparse point; output is created once and never replaces an existing file.
[[nodiscard]] MaterializeResult materialize_capture_summary_v1(
    const std::filesystem::path& capture_path,
    const std::filesystem::path& summary_path) noexcept;

[[nodiscard]] const char* materialize_error_name(MaterializeError error) noexcept;

}  // namespace gore_as_capture::v1::offline
