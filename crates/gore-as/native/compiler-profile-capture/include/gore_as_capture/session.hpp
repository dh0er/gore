#pragma once

#include "gore_as_capture/format.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <memory>
#include <span>
#include <string>

namespace gore_as_capture::v1 {

enum class CaptureError : std::uint32_t {
  ok = 0,
  invalid_argument,
  wrong_target,
  unsafe_output_path,
  output_exists,
  io_error,
  crypto_error,
  size_limit,
  record_limit,
  invalid_state,
  pointer_outside_primary_image,
  duplicate_or_late_record,
  output_recovery_required,
};

struct BuildJitFact final {
  std::uint32_t build_identifier{};
  bool shipping_cache_matches{};
  bool jit_info_present{};
  bool jit_guid_matches{};
  bool jit_database_cleared{};
  bool as_reference_debugging{};
  bool fork_opcode_table_201_212_present{};
  bool reference_debug_opcodes_emittable{};
  bool resolve_object_ptr_callback_registered{};
  GuidBytes precompiled_guid{};
  GuidBytes compiled_jit_guid{};
  std::uint32_t get_build_identifier_rva{};
  std::uint32_t get_static_jit_info_rva{};
};

struct FrontendBoundary final {
  FrontendBoundaryKind kind{};
  std::uint32_t observation_rva{};
  std::uint32_t module_count{};
  std::int32_t result_code{};
  Digest config_sha256{};
  Digest input_sha256{};
  Digest output_sha256{};
};

/// Dormant writer for an authorized in-process instrumentation host.
///
/// The class owns no hooks and never launches or attaches to a process. Every public method
/// fails closed, and a failed session cannot be resumed or sealed. The destructor closes an
/// unsealed file but deliberately does not delete it; an unsealed artifact is rejected offline.
class CaptureSession final {
 public:
  CaptureSession() noexcept;
  ~CaptureSession();
  CaptureSession(CaptureSession&&) noexcept;
  CaptureSession& operator=(CaptureSession&&) noexcept;
  CaptureSession(const CaptureSession&) = delete;
  CaptureSession& operator=(const CaptureSession&) = delete;

  [[nodiscard]] CaptureError open_pinned(
      const std::filesystem::path& executable_path,
      const std::filesystem::path& output_path,
      const void* primary_image_base,
      std::uint64_t observed_steam_build_id,
      const GuidBytes& capture_id) noexcept;

  [[nodiscard]] CaptureError append_engine_property(
      std::uint32_t property_id,
      std::uint64_t value,
      std::uint32_t observation_rva) noexcept;

  /// Returns an opaque token. Only the primary-image RVA is serialized.
  [[nodiscard]] CaptureError intern_primary_image_pointer(
      const void* pointer,
      std::uint32_t& token_out) noexcept;

  [[nodiscard]] CaptureError append_bind_begin(
      std::uint32_t callback_ordinal,
      std::int32_t bind_order,
      std::uint32_t callback_pointer_token,
      const RegistryCounts& counts,
      const Digest& registry_sha256) noexcept;
  [[nodiscard]] CaptureError append_bind_end(
      std::uint32_t callback_ordinal,
      std::int32_t bind_order,
      std::uint32_t callback_pointer_token,
      const RegistryCounts& counts,
      const Digest& registry_sha256) noexcept;

  [[nodiscard]] CaptureError append_registry_delta_json(
      std::span<const std::byte> utf8_json) noexcept;
  [[nodiscard]] CaptureError append_post_bind_mutation_json(
      std::span<const std::byte> utf8_json) noexcept;
  /// May be emitted only after all callbacks and registry-support metadata.
  [[nodiscard]] CaptureError append_registry_support_json(
      std::span<const std::byte> utf8_json) noexcept;
  [[nodiscard]] CaptureError append_final_post_bind_state_json(
      std::span<const std::byte> utf8_json) noexcept;

  [[nodiscard]] CaptureError append_build_jit(const BuildJitFact& fact) noexcept;
  /// `config_kind`: 1 preprocessor, 2 class generator, 3 compiler options.
  [[nodiscard]] CaptureError append_frontend_config_json(
      std::uint32_t config_kind,
      std::span<const std::byte> utf8_json) noexcept;
  [[nodiscard]] CaptureError append_frontend_boundary(
      const FrontendBoundary& boundary) noexcept;

  [[nodiscard]] CaptureError seal() noexcept;
  [[nodiscard]] CaptureError status() const noexcept;
  [[nodiscard]] std::uint64_t record_count() const noexcept;

 private:
  struct Impl;
  std::unique_ptr<Impl> impl_;
};

}  // namespace gore_as_capture::v1
