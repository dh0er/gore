#pragma once

#include "gore_as_capture/instrumentation.hpp"
#include "target_frontend_observer.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <span>

namespace gore_as_capture::v1::instrumentation {

inline constexpr std::size_t kProductionBaseSiteCount = 9;
inline constexpr std::size_t kProductionRegistrationSiteCount = 14;
inline constexpr std::size_t kProductionFrontendSiteCount = 3;
inline constexpr std::size_t kProductionSiteCount =
    kProductionBaseSiteCount + kProductionRegistrationSiteCount +
    kProductionFrontendSiteCount;

enum class ProductionShimPhase : std::uint32_t {
  before = 1,
  after = 2,
};

// ABI shared with production_observer_shims.asm. Offsets are deliberately asserted in the .cpp.
// It is a transient capability frame only: no pointer or register value may enter the wire.
struct alignas(16) ProductionMachineFrame final {
  std::uint64_t rax{};
  std::uint64_t rcx{};
  std::uint64_t rdx{};
  std::uint64_t rbx{};
  std::uint64_t rsp{};
  std::uint64_t rbp{};
  std::uint64_t rsi{};
  std::uint64_t rdi{};
  std::uint64_t r8{};
  std::uint64_t r9{};
  std::uint64_t r10{};
  std::uint64_t r11{};
  std::uint64_t r12{};
  std::uint64_t r13{};
  std::uint64_t r14{};
  std::uint64_t r15{};
  std::uint64_t rflags{};
  std::uint64_t reserved0{};
  std::array<std::array<std::byte, 16>, 16> xmm{};
};

static_assert(alignof(ProductionMachineFrame) == 16);
static_assert(sizeof(ProductionMachineFrame) == 0x190);

using ProductionShimDispatch = bool (*)(
    void* context,
    std::uint32_t site_id,
    ProductionShimPhase phase,
    ProductionMachineFrame& frame) noexcept;

struct ProductionShimObserver final {
  void* context{};
  ProductionShimDispatch dispatch{};
};

enum class ProductionPatchError : std::uint32_t {
  ok = 0,
  invalid_argument,
  invalid_state,
  wrong_thread,
  target_drift,
  thread_in_patch_range,
  allocation_failed,
  relocation_failed,
  protection_failed,
  patch_failed,
  rollback_failed,
  observer_failed,
  active_return_frames,
};

struct ProductionPatchSiteView final {
  std::uint32_t site_id{};
  std::uint32_t patch_rva{};
  std::uint32_t continuation_rva{};
  std::uint8_t overwrite_bytes{};
  bool call_rewrite{};
  bool return_substitution{};
};

// Exact-BuildID in-process transaction. preflight() is read-only. install() performs one
// all-or-nothing write transaction after suspending other threads and checking every RIP. The
// object owns all relays/trampolines and must outlive an installed transaction.
class ProductionPatchCoordinator final {
 public:
  struct Impl;

  ProductionPatchCoordinator() noexcept;
  ~ProductionPatchCoordinator();
  ProductionPatchCoordinator(const ProductionPatchCoordinator&) = delete;
  ProductionPatchCoordinator& operator=(const ProductionPatchCoordinator&) = delete;

  ProductionPatchError preflight(
      std::uintptr_t primary_image,
      std::uint64_t session_id,
      ProductionShimObserver observer) noexcept;
  ProductionPatchError install() noexcept;
  ProductionPatchError uninstall() noexcept;
  ProductionPatchError prepare_unload() const noexcept;

  [[nodiscard]] bool installed() const noexcept { return installed_; }
  [[nodiscard]] std::uintptr_t primary_image() const noexcept { return primary_image_; }
  [[nodiscard]] std::uint64_t session_id() const noexcept { return session_id_; }
  [[nodiscard]] std::span<const ProductionPatchSiteView> sites() const noexcept {
    return sites_;
  }

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  void inject_install_post_write_failure_for_test() noexcept;
  void inject_install_rollback_failure_for_test() noexcept;
  void inject_uninstall_post_write_failure_for_test() noexcept;
  [[nodiscard]] bool validate_initial_compile_unwind_for_test() const noexcept;
  [[nodiscard]] bool validate_relay_unwind_for_test() const noexcept;
  [[nodiscard]] bool validate_unsafe_ranges_for_test() const noexcept;
#endif

 private:
  Impl* impl_{};
  std::uintptr_t primary_image_{};
  std::uint64_t session_id_{};
  std::array<ProductionPatchSiteView, kProductionSiteCount> sites_{};
  bool preflighted_{};
  bool installed_{};
};

// Called only by the MASM entry/return shims. They are exported with C linkage for a stable
// assembler symbol, but are not part of the DLL's public capture ABI.
extern "C" void __cdecl gore_as_capture_production_shim_before(
    ProductionMachineFrame* frame,
    std::uint32_t site_id) noexcept;
extern "C" void __cdecl gore_as_capture_production_shim_after(
    ProductionMachineFrame* frame) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
std::uint32_t production_observer_shims_selftest_stages_v1() noexcept;
bool production_observer_shims_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
