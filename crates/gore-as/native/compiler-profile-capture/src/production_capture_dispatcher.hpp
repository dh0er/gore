#pragma once

#include "production_capture_phase_machine.hpp"
#include "production_observer_shims.hpp"

#include <cstdint>

namespace gore_as_capture::v1::instrumentation {

enum class ProductionCaptureCoordinatorError : std::uint32_t {
  ok = 0,
  invalid_state,
  wrong_thread,
  target_drift,
  semantic_failure,
  patch_failure,
  recovery_required,
  terminal_failure,
};

// Owns the one exact selected-generation capture transaction. preflight() is read-only;
// install()/uninstall() delegate all writes to ProductionPatchCoordinator. The dispatcher is
// direct (one switch over all 26 pinned site IDs) and never exposes a generic patch/inject API.
class ProductionCaptureCoordinator final {
 public:
  struct Impl;

  ProductionCaptureCoordinator() noexcept;
  ~ProductionCaptureCoordinator();
  ProductionCaptureCoordinator(const ProductionCaptureCoordinator&) = delete;
  ProductionCaptureCoordinator& operator=(const ProductionCaptureCoordinator&) = delete;

  ProductionCaptureCoordinatorError preflight(
      std::uint64_t session_id,
      std::uintptr_t primary_image,
      ProductionCaptureSink sink) noexcept;
  ProductionCaptureCoordinatorError install() noexcept;
  ProductionCaptureCoordinatorError uninstall() noexcept;
  ProductionCaptureCoordinatorError prepare_unload() const noexcept;

  [[nodiscard]] bool installed() const noexcept;
  [[nodiscard]] bool recovery_required() const noexcept;
  [[nodiscard]] bool committed() const noexcept;
  [[nodiscard]] bool terminal() const noexcept;

 private:
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  friend bool production_capture_dispatcher_selftest_v1() noexcept;
#endif
  static bool dispatch(
      void* context,
      std::uint32_t site_id,
      ProductionShimPhase phase,
      ProductionMachineFrame& frame) noexcept;

  Impl* impl_{};
};

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool production_capture_dispatcher_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
