#pragma once

#include "gore_as_capture/bridge.h"
#include "target_frontend_observer.hpp"
#include "target_snapshot.hpp"

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

enum class ProductionCapturePhaseError : std::uint32_t {
  ok = 0,
  invalid_argument,
  invalid_order,
  wrong_thread,
  target_drift,
  limit_exceeded,
  sink_rejected,
  terminal_failure,
};

struct ProductionCaptureSink final {
  void* context{};
  bool (*validate)(void*, std::uint64_t, std::uintptr_t) noexcept{};
  std::uint32_t (*intern_pointer)(
      void*, std::uint64_t, std::uintptr_t, std::uint32_t&) noexcept{};
  std::uint32_t (*engine_property)(
      void*, std::uint64_t, std::uint32_t, std::uint64_t) noexcept{};
  std::uint32_t (*bind_begin)(
      void*, std::uint64_t, std::uint32_t, std::int32_t, std::uint32_t,
      const PublicRegistrySnapshot&) noexcept{};
  std::uint32_t (*bind_end)(
      void*, std::uint64_t, std::uint32_t, std::int32_t, std::uint32_t,
      const PublicRegistrySnapshot&) noexcept{};
  std::uint32_t (*json)(
      void*, std::uint64_t, std::uint32_t, const std::string&) noexcept{};
  std::uint32_t (*build_jit)(
      void*, std::uint64_t, const gore_as_capture_build_jit_v1&) noexcept{};
  std::uint32_t (*frontend_config)(
      void*, std::uint64_t, std::uint32_t, const std::string&) noexcept{};
  std::uint32_t (*frontend_boundary)(
      void*, std::uint64_t, const FrontendBoundaryProjection&) noexcept{};
  std::uint32_t (*seal)(void*, std::uint64_t) noexcept{};
  std::uint32_t (*abort)(void*, std::uint64_t) noexcept{};
};

ProductionCaptureSink production_bridge_sink_v1() noexcept;

// Canonical append transaction. Semantic outputs remain buffered until complete(), then are
// emitted once in the only wire-valid order. Any sink refusal triggers exactly one abort and
// makes the object terminal; no partial stream can be sealed by this coordinator.
class ProductionCapturePhaseMachine final {
 public:
  ProductionCapturePhaseError preflight(
      std::uint64_t session_id,
      std::uintptr_t primary_image,
      ProductionCaptureSink sink) noexcept;
  ProductionCapturePhaseError adopt_runtime_owner() noexcept;
  ProductionCapturePhaseError add_engine_property(
      std::uint32_t property_id,
      std::uint64_t value) noexcept;
  ProductionCapturePhaseError intern_primary_image_pointer(
      std::uintptr_t pointer,
      std::uint32_t& token) noexcept;
  ProductionCapturePhaseError begin_bind(
      std::int32_t bind_order,
      std::uint32_t callback_pointer_token,
      const PublicRegistrySnapshot& baseline) noexcept;
  ProductionCapturePhaseError add_registry_delta(std::string json) noexcept;
  ProductionCapturePhaseError end_bind(
      const PublicRegistrySnapshot& final_snapshot) noexcept;
  ProductionCapturePhaseError replace_registry_deltas(
      std::vector<std::vector<std::string>> deltas) noexcept;
  ProductionCapturePhaseError complete_registry(
      std::string support_json,
      std::vector<std::string> final_state_json) noexcept;
  ProductionCapturePhaseError set_build_jit(
      const gore_as_capture_build_jit_v1& fact) noexcept;
  ProductionCapturePhaseError set_frontend(
      std::string preprocessor_json,
      std::string class_generator_json,
      std::string compiler_options_json,
      std::vector<FrontendBoundaryProjection> boundaries) noexcept;
  ProductionCapturePhaseError complete() noexcept;
  ProductionCapturePhaseError abort() noexcept;

  [[nodiscard]] bool terminal() const noexcept { return terminal_; }
  [[nodiscard]] bool committed() const noexcept { return committed_; }
  [[nodiscard]] bool needs_abort() const noexcept {
    return preflighted_ && !committed_ && !abort_complete_;
  }

 private:
  struct Property final {
    std::uint32_t id{};
    std::uint64_t value{};
  };
  struct Bind final {
    std::uint32_t ordinal{};
    std::int32_t order{};
    std::uint32_t callback_token{};
    PublicRegistrySnapshot baseline{};
    PublicRegistrySnapshot final_snapshot{};
    std::vector<std::string> deltas;
  };
  ProductionCapturePhaseError reject(ProductionCapturePhaseError error) noexcept;
  ProductionCapturePhaseError sink_failure() noexcept;
  bool valid_owner() const noexcept;

  std::uint64_t session_id_{};
  std::uintptr_t primary_image_{};
  std::uint32_t owner_thread_{};
  ProductionCaptureSink sink_{};
  std::vector<Property> properties_;
  std::vector<std::uintptr_t> pointer_capabilities_;
  std::vector<Bind> binds_;
  std::string support_json_;
  std::vector<std::string> final_state_json_;
  gore_as_capture_build_jit_v1 build_jit_{};
  std::array<std::string, 3> frontend_json_{};
  std::vector<FrontendBoundaryProjection> boundaries_;
  bool preflighted_{};
  bool bind_active_{};
  bool registry_complete_{};
  bool build_complete_{};
  bool frontend_complete_{};
  bool committed_{};
  bool terminal_{};
  bool abort_complete_{};
};

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool production_capture_phase_machine_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
