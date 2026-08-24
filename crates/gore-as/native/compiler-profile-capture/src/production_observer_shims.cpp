#include "production_observer_shims.hpp"

#include "gore_as_capture/bridge.h"
#include "gore_as_capture/registration_hook_contract.hpp"

#include <windows.h>
#include <tlhelp32.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <mutex>
#include <new>
#include <thread>
#include <utility>
#include <vector>

namespace gore_as_capture::v1::instrumentation {
namespace {

namespace registration = gore_as_capture::v1::instrumentation::registration;

static_assert(offsetof(ProductionMachineFrame, rax) == 0x00);
static_assert(offsetof(ProductionMachineFrame, rsp) == 0x20);
static_assert(offsetof(ProductionMachineFrame, rflags) == 0x80);
static_assert(offsetof(ProductionMachineFrame, xmm) == 0x90);

extern "C" {
void gore_as_capture_production_site_00();
void gore_as_capture_production_site_01();
void gore_as_capture_production_site_02();
void gore_as_capture_production_site_03();
void gore_as_capture_production_site_04();
void gore_as_capture_production_site_05();
void gore_as_capture_production_site_06();
void gore_as_capture_production_site_07();
void gore_as_capture_production_site_08();
void gore_as_capture_production_site_09();
void gore_as_capture_production_site_10();
void gore_as_capture_production_site_11();
void gore_as_capture_production_site_12();
void gore_as_capture_production_site_13();
void gore_as_capture_production_site_14();
void gore_as_capture_production_site_15();
void gore_as_capture_production_site_16();
void gore_as_capture_production_site_17();
void gore_as_capture_production_site_18();
void gore_as_capture_production_site_19();
void gore_as_capture_production_site_20();
void gore_as_capture_production_site_21();
void gore_as_capture_production_site_22();
void gore_as_capture_production_site_23();
void gore_as_capture_production_site_24();
void gore_as_capture_production_site_25();
void gore_as_capture_production_return();
std::uint32_t gore_as_capture_production_shim_state_selftest();
}

using ShimEntry = void (*)();
constexpr std::array<ShimEntry, kProductionSiteCount> kShimEntries{{
    gore_as_capture_production_site_00, gore_as_capture_production_site_01,
    gore_as_capture_production_site_02, gore_as_capture_production_site_03,
    gore_as_capture_production_site_04, gore_as_capture_production_site_05,
    gore_as_capture_production_site_06, gore_as_capture_production_site_07,
    gore_as_capture_production_site_08, gore_as_capture_production_site_09,
    gore_as_capture_production_site_10, gore_as_capture_production_site_11,
    gore_as_capture_production_site_12, gore_as_capture_production_site_13,
    gore_as_capture_production_site_14, gore_as_capture_production_site_15,
    gore_as_capture_production_site_16, gore_as_capture_production_site_17,
    gore_as_capture_production_site_18, gore_as_capture_production_site_19,
    gore_as_capture_production_site_20, gore_as_capture_production_site_21,
    gore_as_capture_production_site_22, gore_as_capture_production_site_23,
    gore_as_capture_production_site_24, gore_as_capture_production_site_25,
}};

constexpr bool requires_return_substitution(const std::uint32_t site_id) noexcept {
  return site_id == 3 || site_id == 4 || site_id == 6 || site_id == 7 ||
         (site_id >= kProductionBaseSiteCount &&
          site_id < kProductionBaseSiteCount + kProductionRegistrationSiteCount) ||
         site_id >= kProductionBaseSiteCount + kProductionRegistrationSiteCount;
}

bool readable_writable_pointer(const std::uintptr_t address, const std::size_t bytes) noexcept {
  if (address == 0 || bytes == 0 || address > std::numeric_limits<std::uintptr_t>::max() - bytes) {
    return false;
  }
  MEMORY_BASIC_INFORMATION region{};
  if (VirtualQuery(reinterpret_cast<const void*>(address), &region, sizeof(region)) !=
          sizeof(region) ||
      region.State != MEM_COMMIT || (region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0) {
    return false;
  }
  const DWORD protection = region.Protect & 0xffu;
  if (protection != PAGE_READWRITE && protection != PAGE_WRITECOPY &&
      protection != PAGE_EXECUTE_READWRITE && protection != PAGE_EXECUTE_WRITECOPY) {
    return false;
  }
  const auto base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
  return base <= address && region.RegionSize >= address - base &&
         bytes <= region.RegionSize - (address - base);
}

struct ActiveDispatch final {
  ProductionShimObserver observer{};
  std::atomic<std::uint32_t> active_dispatches{};
  std::atomic<std::uint32_t> active_return_frames{};
  std::atomic<bool> observer_failed{};
};

class DispatchLease final {
 public:
  explicit DispatchLease(ActiveDispatch* const owner) noexcept : owner_(owner) {
    if (owner_ != nullptr) owner_->active_dispatches.fetch_add(1, std::memory_order_acq_rel);
  }
  ~DispatchLease() {
    if (owner_ != nullptr) owner_->active_dispatches.fetch_sub(1, std::memory_order_acq_rel);
  }
  DispatchLease(const DispatchLease&) = delete;
  DispatchLease& operator=(const DispatchLease&) = delete;

 private:
  ActiveDispatch* owner_{};
};

std::atomic<ActiveDispatch*> g_active_dispatch{};
std::mutex g_patch_coordinator_mutex;
ProductionPatchCoordinator* g_preflight_owner{};

struct ReturnFrame final {
  std::uint32_t site_id{};
  std::uintptr_t substituted_slot{};
  std::uintptr_t original_return{};
  ActiveDispatch* owner{};
};

struct ThreadReturnStack final {
  static constexpr std::size_t kMaximumDepth = 64;
  std::array<ReturnFrame, kMaximumDepth> frames{};
  std::size_t depth{};
};

thread_local ThreadReturnStack g_return_stack{};

bool append_absolute_jump(
    std::byte* const output,
    const std::size_t capacity,
    const std::uintptr_t target,
    std::size_t& bytes_out) noexcept {
  if (output == nullptr || capacity < 14) return false;
  constexpr std::array<std::byte, 6> opcode{
      std::byte{0xff}, std::byte{0x25}, std::byte{0},
      std::byte{0}, std::byte{0}, std::byte{0}};
  std::memcpy(output, opcode.data(), opcode.size());
  std::memcpy(output + opcode.size(), &target, sizeof(target));
  bytes_out = opcode.size() + sizeof(target);
  return true;
}

bool append_stack_home_jump(
    std::byte* const output,
    const std::size_t capacity,
    const std::uintptr_t target,
    const std::uint32_t stack_delta,
    std::size_t& bytes_out) noexcept {
  // The caller-provided first home slot is owned scratch space for these fixed-signature
  // callees. Materializing the destination there preserves every register/RFLAGS and avoids the
  // x64 epilog-only JMP encodings, so asynchronous unwind always applies the relocated prolog.
  constexpr std::size_t bytes = 29;
  if (output == nullptr || capacity < bytes ||
      stack_delta > static_cast<std::uint32_t>(std::numeric_limits<std::int32_t>::max() - 12)) {
    return false;
  }
  const auto home = static_cast<std::int32_t>(stack_delta + 8);
  const auto high_home = static_cast<std::int32_t>(stack_delta + 12);
  const auto low = static_cast<std::uint32_t>(target);
  const auto high = static_cast<std::uint32_t>(target >> 32);
  constexpr std::array<std::byte, 3> store{
      std::byte{0xc7}, std::byte{0x84}, std::byte{0x24}};
  constexpr std::array<std::byte, 3> jump{
      std::byte{0xff}, std::byte{0xa4}, std::byte{0x24}};
  std::size_t cursor = 0;
  std::memcpy(output + cursor, store.data(), store.size());
  cursor += store.size();
  std::memcpy(output + cursor, &home, sizeof(home));
  cursor += sizeof(home);
  std::memcpy(output + cursor, &low, sizeof(low));
  cursor += sizeof(low);
  std::memcpy(output + cursor, store.data(), store.size());
  cursor += store.size();
  std::memcpy(output + cursor, &high_home, sizeof(high_home));
  cursor += sizeof(high_home);
  std::memcpy(output + cursor, &high, sizeof(high));
  cursor += sizeof(high);
  std::memcpy(output + cursor, jump.data(), jump.size());
  cursor += jump.size();
  std::memcpy(output + cursor, &home, sizeof(home));
  cursor += sizeof(home);
  bytes_out = cursor;
  return cursor == bytes;
}

bool registration_stack_delta(
    const registration::RegistrationHookPoint& hook,
    std::uint32_t& delta) noexcept {
  delta = 0;
  for (std::size_t index = 0; index < hook.unwind_operation_count; ++index) {
    const auto& operation = hook.unwind[index];
    std::uint32_t added = 0;
    if (operation.kind == registration::UnwindOperationKind::push_nonvolatile) {
      added = 8;
    } else if (operation.kind == registration::UnwindOperationKind::allocate_stack) {
      added = operation.stack_offset;
    }
    if (delta > std::numeric_limits<std::uint32_t>::max() - added) return false;
    delta += added;
  }
  return delta % 8 == 0;
}

bool relative_displacement(
    const std::uintptr_t instruction_end,
    const std::uintptr_t target,
    std::int32_t& output) noexcept {
  const auto delta = static_cast<std::int64_t>(target) -
                     static_cast<std::int64_t>(instruction_end);
  if (delta < std::numeric_limits<std::int32_t>::min() ||
      delta > std::numeric_limits<std::int32_t>::max()) {
    return false;
  }
  output = static_cast<std::int32_t>(delta);
  return true;
}

bool relocate_rel32(
    std::byte* const instruction,
    const std::uintptr_t original,
    const std::uintptr_t relocated,
    const std::size_t displacement_offset,
    const std::size_t instruction_end_offset) noexcept {
  std::int32_t old_displacement = 0;
  std::memcpy(&old_displacement, instruction + displacement_offset, sizeof(old_displacement));
  const auto target_signed = static_cast<std::int64_t>(original + instruction_end_offset) +
                             old_displacement;
  if (target_signed < 0) return false;
  std::int32_t replacement = 0;
  if (!relative_displacement(
          relocated + instruction_end_offset,
          static_cast<std::uintptr_t>(target_signed),
          replacement)) {
    return false;
  }
  std::memcpy(instruction + displacement_offset, &replacement, sizeof(replacement));
  return true;
}

struct RegistrationUnwindBlob final {
  std::array<std::byte, 64> bytes{};
  std::uint8_t byte_count{};
};

bool build_registration_unwind(
    const registration::RegistrationHookPoint& hook,
    RegistrationUnwindBlob& blob) noexcept {
  constexpr std::uint8_t uwop_allocate_large = 1;
  constexpr std::uint8_t uwop_save_nonvolatile = 4;
  blob = {};
  blob.bytes[0] = std::byte{1};
  blob.bytes[1] = static_cast<std::byte>(hook.overwrite_bytes);
  std::size_t cursor = 4;
  std::uint8_t slots = 0;
  std::uint8_t previous = std::numeric_limits<std::uint8_t>::max();
  for (std::size_t index = 0; index < hook.unwind_operation_count; ++index) {
    const auto& operation = hook.unwind[index];
    if (operation.code_offset == 0 || operation.code_offset > hook.overwrite_bytes ||
        operation.code_offset > previous || cursor > blob.bytes.size() - 4) {
      return false;
    }
    previous = operation.code_offset;
    blob.bytes[cursor++] = static_cast<std::byte>(operation.code_offset);
    switch (operation.kind) {
      case registration::UnwindOperationKind::push_nonvolatile:
        blob.bytes[cursor++] = static_cast<std::byte>(
            static_cast<std::uint8_t>(operation.reg) << 4);
        ++slots;
        break;
      case registration::UnwindOperationKind::save_nonvolatile: {
        if (operation.stack_offset % 8 != 0 || operation.stack_offset / 8 > 0xffff) {
          return false;
        }
        blob.bytes[cursor++] = static_cast<std::byte>(
            (static_cast<std::uint8_t>(operation.reg) << 4) |
            uwop_save_nonvolatile);
        const auto scaled = static_cast<std::uint16_t>(operation.stack_offset / 8);
        std::memcpy(blob.bytes.data() + cursor, &scaled, sizeof(scaled));
        cursor += sizeof(scaled);
        slots = static_cast<std::uint8_t>(slots + 2);
        break;
      }
      case registration::UnwindOperationKind::allocate_stack: {
        if (operation.stack_offset == 0 || operation.stack_offset % 8 != 0 ||
            operation.stack_offset / 8 > 0xffff) {
          return false;
        }
        blob.bytes[cursor++] = static_cast<std::byte>(uwop_allocate_large);
        const auto scaled = static_cast<std::uint16_t>(operation.stack_offset / 8);
        std::memcpy(blob.bytes.data() + cursor, &scaled, sizeof(scaled));
        cursor += sizeof(scaled);
        slots = static_cast<std::uint8_t>(slots + 2);
        break;
      }
      default:
        return false;
    }
  }
  if ((slots & 1u) != 0) cursor += 2;
  if (cursor > blob.bytes.size()) return false;
  blob.bytes[2] = static_cast<std::byte>(slots);
  blob.byte_count = static_cast<std::uint8_t>(cursor);
  return true;
}

bool build_initial_compile_unwind(RegistrationUnwindBlob& blob) noexcept {
  // Relocated bytes: MOV R11,RSP; PUSH RBP; PUSH RBX; LEA RBP,[R11-168h]. Only the two pushes
  // change RSP. Codes are stored in descending prolog-offset order as required by x64 unwind.
  constexpr std::uint8_t uwop_push_nonvolatile = 0;
  constexpr std::uint8_t register_rbx = 3;
  constexpr std::uint8_t register_rbp = 5;
  blob = {};
  blob.bytes[0] = std::byte{1};
  blob.bytes[1] = std::byte{12};
  blob.bytes[2] = std::byte{2};
  blob.bytes[4] = std::byte{5};
  blob.bytes[5] = static_cast<std::byte>((register_rbx << 4) | uwop_push_nonvolatile);
  blob.bytes[6] = std::byte{4};
  blob.bytes[7] = static_cast<std::byte>((register_rbp << 4) | uwop_push_nonvolatile);
  blob.byte_count = 8;
  return true;
}

bool fixed_stack_delta_from_unwind(
    const std::uintptr_t image_base,
    const RUNTIME_FUNCTION& function,
    const std::uint32_t control_offset,
    std::uint32_t& delta,
    const std::uint32_t depth = 0) noexcept {
  constexpr std::uint8_t unw_flag_chaininfo = 4;
  constexpr std::uint8_t uwop_push_nonvolatile = 0;
  constexpr std::uint8_t uwop_allocate_large = 1;
  constexpr std::uint8_t uwop_allocate_small = 2;
  constexpr std::uint8_t uwop_set_frame_pointer = 3;
  constexpr std::uint8_t uwop_save_nonvolatile = 4;
  constexpr std::uint8_t uwop_save_nonvolatile_far = 5;
  constexpr std::uint8_t uwop_save_xmm128 = 8;
  constexpr std::uint8_t uwop_save_xmm128_far = 9;
  constexpr std::uint8_t uwop_push_machine_frame = 10;
  if (image_base == 0 || function.UnwindData == 0 || depth == 8) return false;
  const auto* const info = reinterpret_cast<const std::uint8_t*>(
      image_base + function.UnwindData);
  const auto version = info[0] & 0x7u;
  const auto flags = info[0] >> 3;
  const auto code_count = info[2];
  if ((version != 1 && version != 2) || (flags & unw_flag_chaininfo) != 0 &&
          (flags & ~unw_flag_chaininfo) != 0) {
    return false;
  }
  std::uint32_t added_delta = 0;
  std::size_t slot = 0;
  while (slot < code_count) {
    const auto* const code = info + 4 + slot * 2;
    const auto code_offset = code[0];
    const auto operation = code[1] & 0x0fu;
    const auto operation_info = code[1] >> 4;
    std::size_t slots = 1;
    std::uint32_t stack_bytes = 0;
    switch (operation) {
      case uwop_push_nonvolatile:
        stack_bytes = 8;
        break;
      case uwop_allocate_large:
        if (operation_info == 0) {
          slots = 2;
          if (slot + slots > code_count) return false;
          std::uint16_t scaled = 0;
          std::memcpy(&scaled, code + 2, sizeof(scaled));
          stack_bytes = static_cast<std::uint32_t>(scaled) * 8;
        } else if (operation_info == 1) {
          slots = 3;
          if (slot + slots > code_count) return false;
          std::memcpy(&stack_bytes, code + 2, sizeof(stack_bytes));
        } else {
          return false;
        }
        break;
      case uwop_allocate_small:
        stack_bytes = static_cast<std::uint32_t>(operation_info) * 8 + 8;
        break;
      case uwop_set_frame_pointer:
        // A frame register permits unrecorded dynamic RSP movement. Such a site cannot use the
        // fixed caller-home transfer and is refused rather than guessed.
        return false;
      case uwop_save_nonvolatile:
      case uwop_save_xmm128:
        slots = 2;
        break;
      case uwop_save_nonvolatile_far:
      case uwop_save_xmm128_far:
        slots = 3;
        break;
      case uwop_push_machine_frame:
        stack_bytes = operation_info == 0 ? 40 : operation_info == 1 ? 48 : 0;
        if (stack_bytes == 0) return false;
        break;
      default:
        return false;
    }
    if (slot + slots > code_count) return false;
    if (code_offset <= control_offset) {
      if (added_delta > std::numeric_limits<std::uint32_t>::max() - stack_bytes) {
        return false;
      }
      added_delta += stack_bytes;
    }
    slot += slots;
  }
  if (delta > std::numeric_limits<std::uint32_t>::max() - added_delta) return false;
  delta += added_delta;

  if ((flags & unw_flag_chaininfo) != 0) {
    const auto aligned_slots = (static_cast<std::size_t>(code_count) + 1) & ~std::size_t{1};
    RUNTIME_FUNCTION chained{};
    std::memcpy(&chained, info + 4 + aligned_slots * 2, sizeof(chained));
    if (chained.BeginAddress >= chained.EndAddress) return false;
    return fixed_stack_delta_from_unwind(
        image_base,
        chained,
        std::numeric_limits<std::uint32_t>::max(),
        delta,
        depth + 1);
  }
  return true;
}

class SuspendedThreadWindow final {
 public:
  static constexpr std::size_t kMaximumThreads = 4096;
  ~SuspendedThreadWindow() { release(); }

  bool acquire() noexcept {
    if (thread_count_ != 0) return false;
    const DWORD process_id = GetCurrentProcessId();
    const DWORD current_thread = GetCurrentThreadId();
    const HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    THREADENTRY32 item{};
    item.dwSize = sizeof(item);
    bool enumerating = Thread32First(snapshot, &item) != FALSE;
    while (enumerating) {
      if (item.th32OwnerProcessID == process_id && item.th32ThreadID != current_thread) {
        const HANDLE thread = OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_LIMITED_INFORMATION,
            FALSE,
            item.th32ThreadID);
        if (thread == nullptr) {
          (void)CloseHandle(snapshot);
          release();
          return false;
        }
        if (SuspendThread(thread) == std::numeric_limits<DWORD>::max()) {
          (void)CloseHandle(thread);
          (void)CloseHandle(snapshot);
          release();
          return false;
        }
        if (thread_count_ == threads_.size()) {
          (void)ResumeThread(thread);
          (void)CloseHandle(thread);
          (void)CloseHandle(snapshot);
          release();
          return false;
        }
        threads_[thread_count_++] = {thread, item.th32ThreadID};
      }
      item.dwSize = sizeof(item);
      enumerating = Thread32Next(snapshot, &item) != FALSE;
    }
    const bool complete = GetLastError() == ERROR_NO_MORE_FILES;
    (void)CloseHandle(snapshot);
    if (!complete || !stable_thread_set(process_id, current_thread)) {
      release();
      return false;
    }
    active_ = true;
    return true;
  }

  bool outside(const std::span<const std::pair<std::uintptr_t, std::uintptr_t>> ranges) const
      noexcept {
    if (!active_) return false;
    for (std::size_t index = 0; index < thread_count_; ++index) {
      const auto& thread = threads_[index];
      CONTEXT context{};
      context.ContextFlags = CONTEXT_CONTROL;
      if (GetThreadContext(thread.handle, &context) == FALSE) return false;
      const auto rip = static_cast<std::uintptr_t>(context.Rip);
      for (const auto& [begin, end] : ranges) {
        if (begin >= end || (rip >= begin && rip < end)) return false;
      }
    }
    return true;
  }

 private:
  struct Thread final { HANDLE handle{}; DWORD id{}; };

  bool stable_thread_set(const DWORD process_id, const DWORD current_thread) const noexcept {
    const HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    THREADENTRY32 item{};
    item.dwSize = sizeof(item);
    bool enumerating = Thread32First(snapshot, &item) != FALSE;
    bool stable = true;
    while (enumerating) {
      if (item.th32OwnerProcessID == process_id && item.th32ThreadID != current_thread &&
           std::none_of(threads_.begin(), threads_.begin() + thread_count_, [&](const Thread& thread) {
            return thread.id == item.th32ThreadID;
          })) {
        stable = false;
        break;
      }
      item.dwSize = sizeof(item);
      enumerating = Thread32Next(snapshot, &item) != FALSE;
    }
    const bool complete = !enumerating && GetLastError() == ERROR_NO_MORE_FILES;
    (void)CloseHandle(snapshot);
    return stable && complete;
  }

  void release() noexcept {
    for (std::size_t index = thread_count_; index > 0; --index) {
      (void)ResumeThread(threads_[index - 1].handle);
      (void)CloseHandle(threads_[index - 1].handle);
      threads_[index - 1] = {};
    }
    thread_count_ = 0;
    active_ = false;
  }

  std::array<Thread, kMaximumThreads> threads_{};
  std::size_t thread_count_{};
  bool active_{};
};

}  // namespace

extern "C" std::uintptr_t
    gore_as_capture_production_shim_targets[kProductionSiteCount]{};

struct ProductionPatchCoordinator::Impl final {
  static constexpr std::size_t kBlockBytes = 256 * 1024;
  static constexpr std::size_t kRelayBase = 0x1000;
  static constexpr std::size_t kRelayStride = 0x1000;
  static constexpr std::size_t kTrampolineStride = 80;
  static constexpr std::size_t kTrampolineBase = 0x20000;
  static constexpr std::size_t kUnwindBase = 0x22000;
  static constexpr std::size_t kUnwindStride = 64;
  static constexpr std::size_t kRelayUnwind =
      kUnwindBase + (kProductionRegistrationSiteCount + 1) * kUnwindStride;
  static constexpr std::size_t kRelayFunctionCount =
      kProductionBaseSiteCount + kProductionFrontendSiteCount;

  struct Plan final {
    std::uintptr_t source{};
    std::uint32_t patch_rva{};
    std::uint8_t length{};
    std::array<std::byte, 24> expected{};
    std::array<std::byte, 24> replacement{};
    std::uintptr_t relay{};
    std::uintptr_t trampoline{};
    std::uint8_t trampoline_bytes{};
  };

  ~Impl() {
    if (relay_function_table_registered) {
      (void)RtlDeleteFunctionTable(relay_unwind_functions.data());
    }
    if (generated_function_table_registered) {
      (void)RtlDeleteFunctionTable(generated_unwind_functions.data());
    }
    if (block != nullptr) (void)VirtualFree(block, 0, MEM_RELEASE);
  }

  bool allocate_near(const std::uintptr_t image) noexcept {
    constexpr std::uintptr_t granularity = 64 * 1024;
    constexpr std::uintptr_t maximum_distance = 0x7fff'0000ull;
    const auto aligned = image & ~(granularity - 1);
    for (std::uintptr_t distance = granularity; distance <= maximum_distance;
         distance += granularity) {
      if (aligned > std::numeric_limits<std::uintptr_t>::max() - distance) continue;
      const auto candidate = aligned + distance;
      block = static_cast<std::byte*>(VirtualAlloc(
          reinterpret_cast<void*>(candidate),
          kBlockBytes,
          MEM_RESERVE | MEM_COMMIT,
          PAGE_READWRITE));
      if (block != nullptr) return true;
    }
    return false;
  }

  std::byte* relay_slot(const std::size_t index) const noexcept {
    return block + kRelayBase + index * kRelayStride;
  }
  std::byte* trampoline(const std::size_t index) const noexcept {
    return block + kTrampolineBase + index * kTrampolineStride;
  }

  bool add_relay(
      const std::size_t site_id,
      const std::uintptr_t primary_image,
      const std::uintptr_t source,
      const bool call_rewrite,
      const bool function_entry,
      const std::uintptr_t destination,
      std::uintptr_t& relay_out) noexcept {
    if (site_id >= kProductionSiteCount || relay_function_count == relay_unwind_functions.size()) {
      return false;
    }
    const auto slot = reinterpret_cast<std::uintptr_t>(relay_slot(site_id));
    if (slot < primary_image || slot - primary_image > std::numeric_limits<DWORD>::max()) {
      return false;
    }

    DWORD unwind_data = 0;
    std::uintptr_t source_offset = 1;
    const auto empty_unwind = reinterpret_cast<std::uintptr_t>(block + kRelayUnwind);
    if (empty_unwind < primary_image ||
        empty_unwind - primary_image > std::numeric_limits<DWORD>::max()) {
      return false;
    }
    if (call_rewrite) {
      unwind_data = static_cast<DWORD>(empty_unwind - primary_image);
    } else {
      DWORD64 source_image = 0;
      const auto* source_function = RtlLookupFunctionEntry(
          static_cast<DWORD64>(source), &source_image, nullptr);
      if (source_function == nullptr || source_image != primary_image ||
          source_function->BeginAddress >= source_function->EndAddress ||
          source < primary_image + source_function->BeginAddress) {
        if (function_entry) {
          source_offset = 0;
          unwind_data = static_cast<DWORD>(empty_unwind - primary_image);
        } else {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
          // Dispatcher-only fixtures never execute their relays and deliberately omit a PE
          // exception directory. The production-shim fixture supplies real source metadata and
          // exercises the exact inline-unwind path below.
          source_offset = 0;
          unwind_data = static_cast<DWORD>(empty_unwind - primary_image);
#else
          return false;
#endif
        }
      } else {
        source_offset = source - (primary_image + source_function->BeginAddress);
        if (source_offset >= source_function->EndAddress - source_function->BeginAddress) {
          return false;
        }
        unwind_data = source_function->UnwindData;
      }
    }
    if (source_offset > kRelayStride - 14) return false;

    relay_out = slot + source_offset;
    std::size_t relay_bytes = 0;
    bool emitted = false;
    if (!call_rewrite && source_offset != 0) {
      std::uint32_t stack_delta = 0;
      DWORD64 source_image = 0;
      const auto* source_function = RtlLookupFunctionEntry(
          static_cast<DWORD64>(source), &source_image, nullptr);
      emitted = source_function != nullptr && source_image == primary_image &&
                fixed_stack_delta_from_unwind(
                    primary_image,
                    *source_function,
                    static_cast<std::uint32_t>(source_offset),
                    stack_delta) &&
                append_stack_home_jump(
                    reinterpret_cast<std::byte*>(relay_out),
                    kRelayStride - source_offset,
                    destination,
                    stack_delta,
                    relay_bytes);
    } else {
      emitted = append_absolute_jump(
          reinterpret_cast<std::byte*>(relay_out),
          kRelayStride - source_offset,
          destination,
          relay_bytes);
    }
    if (!emitted || relay_bytes == 0) {
      return false;
    }
    const auto begin = slot - primary_image;
    if (begin > std::numeric_limits<DWORD>::max() ||
        source_offset + relay_bytes > std::numeric_limits<DWORD>::max() - begin) {
      return false;
    }
    auto& function = relay_unwind_functions[relay_function_count++];
    function.BeginAddress = static_cast<DWORD>(begin);
    function.EndAddress = static_cast<DWORD>(begin + source_offset + relay_bytes);
    function.UnwindData = unwind_data;
    return true;
  }

  std::array<Plan, kProductionSiteCount> plans{};
  std::array<RUNTIME_FUNCTION, kRelayFunctionCount> relay_unwind_functions{};
  std::array<RUNTIME_FUNCTION, kProductionRegistrationSiteCount + 1>
      generated_unwind_functions{};
  std::array<DWORD, kProductionSiteCount> original_protections{};
  std::byte* block{};
  ActiveDispatch active{};
  DWORD owner_thread{};
  std::size_t relay_function_count{};
  std::size_t original_protection_count{};
  bool dispatch_published{};
  bool target_writes_started{};
  bool recovery_required{};
  bool relay_function_table_registered{};
  bool generated_function_table_registered{};
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  bool fail_install_post_write{};
  bool fail_install_rollback{};
  bool fail_uninstall_post_write{};
#endif
};

ProductionPatchCoordinator::ProductionPatchCoordinator() noexcept = default;

ProductionPatchCoordinator::~ProductionPatchCoordinator() {
  if (!installed_) {
    std::scoped_lock coordinator_lock(g_patch_coordinator_mutex);
    if (g_preflight_owner == this) {
      std::fill(std::begin(gore_as_capture_production_shim_targets),
                std::end(gore_as_capture_production_shim_targets), 0);
      g_preflight_owner = nullptr;
    }
    delete impl_;
  }
}

ProductionPatchError ProductionPatchCoordinator::preflight(
    const std::uintptr_t primary_image,
    const std::uint64_t session_id,
    const ProductionShimObserver observer) noexcept {
  std::scoped_lock coordinator_lock(g_patch_coordinator_mutex);
  if (primary_image == 0 || session_id == 0 || observer.dispatch == nullptr) {
    return ProductionPatchError::invalid_argument;
  }
  if (preflighted_ || installed_ || impl_ != nullptr || g_active_dispatch.load() != nullptr ||
      g_preflight_owner != nullptr) {
    return ProductionPatchError::invalid_state;
  }
  auto* candidate = new (std::nothrow) Impl();
  if (candidate == nullptr) return ProductionPatchError::allocation_failed;
  if (!candidate->allocate_near(primary_image)) {
    delete candidate;
    return ProductionPatchError::allocation_failed;
  }
  candidate->block[Impl::kRelayUnwind] = std::byte{1};
  candidate->active.observer = observer;
  candidate->owner_thread = GetCurrentThreadId();
  SYSTEM_INFO system{};
  GetSystemInfo(&system);
  const auto page_bytes = static_cast<std::uintptr_t>(system.dwPageSize);
  if (page_bytes == 0) {
    delete candidate;
    return ProductionPatchError::protection_failed;
  }

  for (std::size_t index = 0; index < kProductionSiteCount; ++index) {
    auto& plan = candidate->plans[index];
    plan.trampoline = reinterpret_cast<std::uintptr_t>(candidate->trampoline(index));

    if (index < kProductionBaseSiteCount) {
      const auto& site = kPinnedInstructionSpans[index];
      plan.patch_rva = site.patch_anchor_rva;
      plan.length = site.byte_count;
      std::copy(site.expected.begin(), site.expected.end(), plan.expected.begin());
      plan.source = primary_image + site.patch_anchor_rva;
      plan.replacement.fill(std::byte{0x90});
      plan.replacement[0] = kStaticSiteContracts[index].transfer_kind ==
                                    GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1
                                ? std::byte{0xe8}
                                : std::byte{0xe9};
      if (!candidate->add_relay(
              index,
              primary_image,
              plan.source,
              kStaticSiteContracts[index].transfer_kind ==
                  GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1,
              kStaticSiteContracts[index].transfer_kind ==
                  GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1,
              reinterpret_cast<std::uintptr_t>(kShimEntries[index]),
              plan.relay)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      std::int32_t displacement = 0;
      if (!relative_displacement(plan.source + 5, plan.relay, displacement)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      std::memcpy(plan.replacement.data() + 1, &displacement, sizeof(displacement));

      std::memcpy(candidate->trampoline(index), site.expected.data(), site.byte_count);
      std::size_t trampoline_bytes = site.byte_count;
      if (index == 0 && !relocate_rel32(
                            candidate->trampoline(index), plan.source, plan.trampoline, 7, 11)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      if (index == 4 && !relocate_rel32(
                            candidate->trampoline(index), plan.source, plan.trampoline, 3, 7)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      if ((index == 6 || index == 7) && !relocate_rel32(
                                              candidate->trampoline(index),
                                              plan.source,
                                              plan.trampoline,
                                              1,
                                              5)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      if (index != 4 && index != 6 && index != 7) {
        std::size_t tail = 0;
        const bool appended = index == 5
                                  ? append_stack_home_jump(
                                        candidate->trampoline(index) + trampoline_bytes,
                                        Impl::kTrampolineStride - trampoline_bytes,
                                        plan.source + plan.length,
                                        16,
                                        tail)
                                  : append_absolute_jump(
                                        candidate->trampoline(index) + trampoline_bytes,
                                        Impl::kTrampolineStride - trampoline_bytes,
                                        plan.source + plan.length,
                                        tail);
        if (!appended) {
          delete candidate;
          return ProductionPatchError::relocation_failed;
        }
        trampoline_bytes += tail;
      }
      plan.trampoline_bytes = static_cast<std::uint8_t>(trampoline_bytes);
      if (index == 6 || index == 7) {
        gore_as_capture_production_shim_targets[index] =
            primary_image + kStaticSiteContracts[index].direct_callee_rva;
      } else {
        gore_as_capture_production_shim_targets[index] = plan.trampoline;
      }
    } else if (index < kProductionBaseSiteCount + kProductionRegistrationSiteCount) {
      const std::size_t registration_index = index - kProductionBaseSiteCount;
      const auto& site = registration::kPinnedRegistrationHooks[registration_index];
      plan.patch_rva = site.function_rva;
      plan.length = site.overwrite_bytes;
      std::copy(site.expected.begin(), site.expected.end(), plan.expected.begin());
      plan.source = primary_image + site.function_rva;
      plan.replacement.fill(std::byte{0x90});
      std::size_t patch_bytes = 0;
      if (!append_absolute_jump(
              plan.replacement.data(), plan.length,
              reinterpret_cast<std::uintptr_t>(kShimEntries[index]), patch_bytes) ||
          patch_bytes > plan.length) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      std::memcpy(candidate->trampoline(index), site.expected.data(), site.overwrite_bytes);
      std::size_t tail = 0;
      std::uint32_t stack_delta = 0;
      if (!registration_stack_delta(site, stack_delta) ||
          !append_stack_home_jump(
              candidate->trampoline(index) + site.overwrite_bytes,
              Impl::kTrampolineStride - site.overwrite_bytes,
              plan.source + site.overwrite_bytes,
              stack_delta,
              tail)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      plan.trampoline_bytes = static_cast<std::uint8_t>(site.overwrite_bytes + tail);
      gore_as_capture_production_shim_targets[index] = plan.trampoline;
    } else {
      const std::size_t frontend_index =
          index - kProductionBaseSiteCount - kProductionRegistrationSiteCount;
      const auto& site = frontend_target_layout::callback_callsites[frontend_index];
      plan.patch_rva = site.call_rva;
      plan.length = static_cast<std::uint8_t>(site.expected_call.size());
      std::copy(site.expected_call.begin(), site.expected_call.end(), plan.expected.begin());
      plan.source = primary_image + site.call_rva;
      plan.replacement.fill(std::byte{0x90});
      plan.replacement[0] = std::byte{0xe8};
      if (!candidate->add_relay(
              index,
              primary_image,
              plan.source,
              true,
              false,
              reinterpret_cast<std::uintptr_t>(kShimEntries[index]),
              plan.relay)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      std::int32_t displacement = 0;
      if (!relative_displacement(plan.source + 5, plan.relay, displacement)) {
        delete candidate;
        return ProductionPatchError::relocation_failed;
      }
      std::memcpy(plan.replacement.data() + 1, &displacement, sizeof(displacement));
      gore_as_capture_production_shim_targets[index] =
          primary_image + site.direct_callee_rva;
    }

    if (plan.source % page_bytes > page_bytes - plan.length ||
        std::memcmp(
            reinterpret_cast<const void*>(plan.source),
            plan.expected.data(),
            plan.length) != 0) {
      std::fill(std::begin(gore_as_capture_production_shim_targets),
                std::end(gore_as_capture_production_shim_targets), 0);
      delete candidate;
      return ProductionPatchError::target_drift;
    }
    sites_[index] = ProductionPatchSiteView{
        static_cast<std::uint32_t>(index),
        plan.patch_rva,
        plan.patch_rva + plan.length,
        plan.length,
        index < kProductionBaseSiteCount
            ? kStaticSiteContracts[index].transfer_kind ==
                  GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1
            : index >= kProductionBaseSiteCount + kProductionRegistrationSiteCount,
        requires_return_substitution(static_cast<std::uint32_t>(index)),
    };
  }

  RegistrationUnwindBlob initial_compile_unwind{};
  if (!build_initial_compile_unwind(initial_compile_unwind)) {
    std::fill(std::begin(gore_as_capture_production_shim_targets),
              std::end(gore_as_capture_production_shim_targets), 0);
    delete candidate;
    return ProductionPatchError::relocation_failed;
  }
  std::memcpy(
      candidate->block + Impl::kUnwindBase,
      initial_compile_unwind.bytes.data(),
      initial_compile_unwind.byte_count);
  candidate->generated_unwind_functions[0].BeginAddress = static_cast<DWORD>(
      Impl::kTrampolineBase + 5 * Impl::kTrampolineStride);
  candidate->generated_unwind_functions[0].EndAddress =
      candidate->generated_unwind_functions[0].BeginAddress +
      candidate->plans[5].trampoline_bytes;
  candidate->generated_unwind_functions[0].UnwindData =
      static_cast<DWORD>(Impl::kUnwindBase);

  for (std::size_t index = 0; index < kProductionRegistrationSiteCount; ++index) {
    RegistrationUnwindBlob unwind{};
    if (!build_registration_unwind(registration::kPinnedRegistrationHooks[index], unwind) ||
        unwind.byte_count == 0) {
      std::fill(std::begin(gore_as_capture_production_shim_targets),
                std::end(gore_as_capture_production_shim_targets), 0);
      delete candidate;
      return ProductionPatchError::relocation_failed;
    }
    const auto site_id = kProductionBaseSiteCount + index;
    std::memcpy(
        candidate->block + Impl::kUnwindBase + (index + 1) * Impl::kUnwindStride,
        unwind.bytes.data(), unwind.byte_count);
    auto& function = candidate->generated_unwind_functions[index + 1];
    function.BeginAddress = static_cast<DWORD>(
        Impl::kTrampolineBase + site_id * Impl::kTrampolineStride);
    function.EndAddress = function.BeginAddress + candidate->plans[site_id].trampoline_bytes;
    function.UnwindData =
        static_cast<DWORD>(Impl::kUnwindBase + (index + 1) * Impl::kUnwindStride);
  }
  DWORD ignored = 0;
  if (VirtualProtect(candidate->block, Impl::kBlockBytes, PAGE_EXECUTE_READ, &ignored) == FALSE ||
      FlushInstructionCache(GetCurrentProcess(), candidate->block, Impl::kBlockBytes) == FALSE) {
    std::fill(std::begin(gore_as_capture_production_shim_targets),
              std::end(gore_as_capture_production_shim_targets), 0);
    delete candidate;
    return ProductionPatchError::protection_failed;
  }
  if (candidate->relay_function_count != candidate->relay_unwind_functions.size() ||
      RtlAddFunctionTable(
          candidate->generated_unwind_functions.data(),
          static_cast<DWORD>(candidate->generated_unwind_functions.size()),
          reinterpret_cast<DWORD64>(candidate->block)) == FALSE) {
    std::fill(std::begin(gore_as_capture_production_shim_targets),
              std::end(gore_as_capture_production_shim_targets), 0);
    delete candidate;
    return ProductionPatchError::relocation_failed;
  }
  candidate->generated_function_table_registered = true;
  if (RtlAddFunctionTable(
          candidate->relay_unwind_functions.data(),
          static_cast<DWORD>(candidate->relay_unwind_functions.size()),
          static_cast<DWORD64>(primary_image)) == FALSE) {
    std::fill(std::begin(gore_as_capture_production_shim_targets),
              std::end(gore_as_capture_production_shim_targets), 0);
    delete candidate;
    return ProductionPatchError::relocation_failed;
  }
  candidate->relay_function_table_registered = true;
  for (const auto& function : candidate->generated_unwind_functions) {
    DWORD64 discovered_base = 0;
    const auto* discovered = RtlLookupFunctionEntry(
        reinterpret_cast<DWORD64>(candidate->block) + function.BeginAddress,
        &discovered_base,
        nullptr);
    if (discovered == nullptr ||
        discovered_base != reinterpret_cast<DWORD64>(candidate->block) ||
        discovered->BeginAddress != function.BeginAddress ||
        discovered->EndAddress != function.EndAddress ||
        discovered->UnwindData != function.UnwindData) {
      std::fill(std::begin(gore_as_capture_production_shim_targets),
                std::end(gore_as_capture_production_shim_targets), 0);
      delete candidate;
      return ProductionPatchError::relocation_failed;
    }
  }
  for (const auto& function : candidate->relay_unwind_functions) {
    DWORD64 discovered_base = 0;
    const auto* discovered = RtlLookupFunctionEntry(
        static_cast<DWORD64>(primary_image) + function.BeginAddress,
        &discovered_base,
        nullptr);
    if (discovered == nullptr || discovered_base != primary_image ||
        discovered->BeginAddress != function.BeginAddress ||
        discovered->EndAddress != function.EndAddress ||
        discovered->UnwindData != function.UnwindData) {
      std::fill(std::begin(gore_as_capture_production_shim_targets),
                std::end(gore_as_capture_production_shim_targets), 0);
      delete candidate;
      return ProductionPatchError::relocation_failed;
    }
  }
  impl_ = candidate;
  primary_image_ = primary_image;
  session_id_ = session_id;
  preflighted_ = true;
  g_preflight_owner = this;
  return ProductionPatchError::ok;
}

namespace {

ProductionPatchError change_all_protections(
    std::span<ProductionPatchCoordinator::Impl::Plan> plans,
    std::array<DWORD, kProductionSiteCount>& old,
    std::size_t& changed) noexcept {
  changed = 0;
  for (; changed < plans.size(); ++changed) {
    if (VirtualProtect(
            reinterpret_cast<void*>(plans[changed].source),
            plans[changed].length,
            PAGE_EXECUTE_READWRITE,
            &old[changed]) == FALSE) {
      return ProductionPatchError::protection_failed;
    }
  }
  return ProductionPatchError::ok;
}

bool make_all_writable(
    std::span<ProductionPatchCoordinator::Impl::Plan> plans) noexcept {
  for (const auto& plan : plans) {
    DWORD ignored = 0;
    if (VirtualProtect(
            reinterpret_cast<void*>(plan.source),
            plan.length,
            PAGE_EXECUTE_READWRITE,
            &ignored) == FALSE) {
      return false;
    }
  }
  return true;
}

bool restore_protections(
    std::span<ProductionPatchCoordinator::Impl::Plan> plans,
    const std::array<DWORD, kProductionSiteCount>& old,
    const std::size_t count) noexcept {
  if (count > plans.size()) return false;
  bool restored = true;
  for (std::size_t index = count; index > 0; --index) {
    DWORD ignored = 0;
    restored = VirtualProtect(
                   reinterpret_cast<void*>(plans[index - 1].source),
                   plans[index - 1].length,
                   old[index - 1], &ignored) != FALSE &&
               restored;
  }
  return restored;
}

constexpr std::size_t kPatchRangeCount =
    kProductionSiteCount * 2 + 4 + kUnsafeInstallRanges.size();

bool patch_ranges(
    const ProductionPatchCoordinator::Impl& impl,
    std::array<std::pair<std::uintptr_t, std::uintptr_t>, kPatchRangeCount>& ranges) noexcept {
  std::size_t count = 0;
  const auto append = [&](const std::uintptr_t begin, const std::uintptr_t end) {
    if (count == ranges.size() || begin >= end) return false;
    ranges[count++] = {begin, end};
    return true;
  };
  for (const auto& plan : impl.plans) {
    if (!append(plan.source, plan.source + plan.length)) return false;
  }
  if (!append(
          reinterpret_cast<std::uintptr_t>(impl.block),
          reinterpret_cast<std::uintptr_t>(impl.block) + impl.kBlockBytes)) {
    return false;
  }
  const auto append_unwind_range = [&](const DWORD64 address) {
    DWORD64 image_base = 0;
    const auto* function = RtlLookupFunctionEntry(address, &image_base, nullptr);
    return function != nullptr && image_base != 0 && function->BeginAddress < function->EndAddress &&
           append(
               static_cast<std::uintptr_t>(image_base + function->BeginAddress),
               static_cast<std::uintptr_t>(image_base + function->EndAddress));
  };
  for (const auto entry : kShimEntries) {
    if (!append_unwind_range(reinterpret_cast<DWORD64>(entry))) return false;
  }
  if (!append_unwind_range(reinterpret_cast<DWORD64>(gore_as_capture_production_return)) ||
      !append_unwind_range(
          reinterpret_cast<DWORD64>(gore_as_capture_production_shim_before)) ||
      !append_unwind_range(
          reinterpret_cast<DWORD64>(gore_as_capture_production_shim_after))) {
    return false;
  }
  for (const auto& range : kUnsafeInstallRanges) {
    if (!append(
            impl.plans[0].source - impl.plans[0].patch_rva + range.begin_rva,
            impl.plans[0].source - impl.plans[0].patch_rva + range.end_rva)) {
      return false;
    }
  }
  return count == ranges.size();
}

bool plan_has_bytes(
    const ProductionPatchCoordinator::Impl::Plan& plan,
    const std::array<std::byte, 24>& bytes) noexcept {
  return std::memcmp(
             reinterpret_cast<const void*>(plan.source), bytes.data(), plan.length) == 0;
}

}  // namespace

ProductionPatchError ProductionPatchCoordinator::install() noexcept {
  std::scoped_lock coordinator_lock(g_patch_coordinator_mutex);
  if (!preflighted_ || installed_ || impl_ == nullptr) return ProductionPatchError::invalid_state;
  if (impl_->owner_thread != GetCurrentThreadId()) return ProductionPatchError::wrong_thread;
  if (impl_->active.observer_failed.load() || g_active_dispatch.load() != nullptr) {
    return ProductionPatchError::invalid_state;
  }
  for (const auto& plan : impl_->plans) {
    if (std::memcmp(reinterpret_cast<const void*>(plan.source), plan.expected.data(), plan.length) !=
        0) {
      return ProductionPatchError::target_drift;
    }
  }
  std::array<std::pair<std::uintptr_t, std::uintptr_t>, kPatchRangeCount> ranges{};
  if (!patch_ranges(*impl_, ranges)) {
    return ProductionPatchError::relocation_failed;
  }
  SuspendedThreadWindow window;
  if (!window.acquire()) return ProductionPatchError::thread_in_patch_range;
  if (!window.outside(ranges)) return ProductionPatchError::thread_in_patch_range;
  for (const auto& plan : impl_->plans) {
    if (!plan_has_bytes(plan, plan.expected)) return ProductionPatchError::target_drift;
  }

  std::array<DWORD, kProductionSiteCount> old{};
  std::size_t changed = 0;
  const auto protection = change_all_protections(impl_->plans, old, changed);
  impl_->original_protections = old;
  impl_->original_protection_count = changed;
  if (protection != ProductionPatchError::ok) {
    if (restore_protections(impl_->plans, impl_->original_protections, changed)) {
      impl_->original_protection_count = 0;
      return protection;
    }
    impl_->recovery_required = true;
    installed_ = true;
    return ProductionPatchError::rollback_failed;
  }
  ActiveDispatch* expected = nullptr;
  if (!g_active_dispatch.compare_exchange_strong(
          expected, &impl_->active, std::memory_order_acq_rel)) {
    if (restore_protections(
            impl_->plans, impl_->original_protections, impl_->original_protection_count)) {
      impl_->original_protection_count = 0;
      return ProductionPatchError::invalid_state;
    }
    impl_->recovery_required = true;
    installed_ = true;
    return ProductionPatchError::rollback_failed;
  }
  impl_->dispatch_published = true;
  for (const auto& plan : impl_->plans) {
    std::memcpy(reinterpret_cast<void*>(plan.source), plan.replacement.data(), plan.length);
  }
  impl_->target_writes_started = true;
  bool flushed = FlushInstructionCache(
                     GetCurrentProcess(), reinterpret_cast<void*>(primary_image_),
                     kPeSizeOfImage) != FALSE;
  bool protections_restored = restore_protections(
      impl_->plans, impl_->original_protections, impl_->original_protection_count);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (impl_->fail_install_post_write) {
    impl_->fail_install_post_write = false;
    DWORD ignored = 0;
    (void)VirtualProtect(
        reinterpret_cast<void*>(impl_->plans[0].source),
        impl_->plans[0].length,
        PAGE_EXECUTE_READWRITE,
        &ignored);
    protections_restored = false;
    flushed = false;
  }
#endif
  if (!flushed || !protections_restored) {
    bool writable = make_all_writable(impl_->plans);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
    if (impl_->fail_install_rollback) {
      impl_->fail_install_rollback = false;
      writable = false;
    }
#endif
    if (writable) {
      for (auto iterator = impl_->plans.rbegin(); iterator != impl_->plans.rend(); ++iterator) {
        std::memcpy(
            reinterpret_cast<void*>(iterator->source),
            iterator->expected.data(),
            iterator->length);
      }
    }
    const bool rollback_flush = writable &&
                                FlushInstructionCache(
                                    GetCurrentProcess(),
                                    reinterpret_cast<void*>(primary_image_),
                                    kPeSizeOfImage) != FALSE;
    const bool rollback_protection = restore_protections(
        impl_->plans, impl_->original_protections, impl_->original_protection_count);
    ActiveDispatch* active = &impl_->active;
    const bool unpublished = writable && rollback_flush && rollback_protection &&
                             g_active_dispatch.compare_exchange_strong(
                                 active, nullptr, std::memory_order_acq_rel);
    if (unpublished) {
      impl_->dispatch_published = false;
      impl_->target_writes_started = false;
      impl_->original_protection_count = 0;
      return ProductionPatchError::patch_failed;
    }
    impl_->recovery_required = true;
    installed_ = true;
    return ProductionPatchError::rollback_failed;
  }
  installed_ = true;
  return ProductionPatchError::ok;
}

ProductionPatchError ProductionPatchCoordinator::uninstall() noexcept {
  std::scoped_lock coordinator_lock(g_patch_coordinator_mutex);
  if (!installed_ || impl_ == nullptr) return ProductionPatchError::invalid_state;
  if (impl_->owner_thread != GetCurrentThreadId()) return ProductionPatchError::wrong_thread;
  if (impl_->active.active_return_frames.load() != 0 ||
      impl_->active.active_dispatches.load() != 0) {
    return ProductionPatchError::active_return_frames;
  }
  for (const auto& plan : impl_->plans) {
    if (!plan_has_bytes(plan, plan.replacement) && !plan_has_bytes(plan, plan.expected)) {
      return ProductionPatchError::target_drift;
    }
  }
  std::array<std::pair<std::uintptr_t, std::uintptr_t>, kPatchRangeCount> ranges{};
  if (!patch_ranges(*impl_, ranges)) {
    return ProductionPatchError::relocation_failed;
  }
  SuspendedThreadWindow window;
  if (!window.acquire()) return ProductionPatchError::thread_in_patch_range;
  if (!window.outside(ranges)) return ProductionPatchError::thread_in_patch_range;
  if (impl_->active.active_return_frames.load(std::memory_order_acquire) != 0 ||
      impl_->active.active_dispatches.load(std::memory_order_acquire) != 0) {
    return ProductionPatchError::active_return_frames;
  }

  bool has_replacement = false;
  for (const auto& plan : impl_->plans) {
    const bool replacement = plan_has_bytes(plan, plan.replacement);
    if (!replacement && !plan_has_bytes(plan, plan.expected)) {
      return ProductionPatchError::target_drift;
    }
    has_replacement = has_replacement || replacement;
  }
  const bool writable = !has_replacement || make_all_writable(impl_->plans);
  if (writable && has_replacement) {
    for (auto iterator = impl_->plans.rbegin(); iterator != impl_->plans.rend(); ++iterator) {
      std::memcpy(
          reinterpret_cast<void*>(iterator->source),
          iterator->expected.data(),
          iterator->length);
    }
  }
  bool flushed = !has_replacement ||
                 (writable && FlushInstructionCache(
                                  GetCurrentProcess(),
                                  reinterpret_cast<void*>(primary_image_),
                                  kPeSizeOfImage) != FALSE);
  bool protections_restored = restore_protections(
      impl_->plans, impl_->original_protections, impl_->original_protection_count);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (impl_->fail_uninstall_post_write) {
    impl_->fail_uninstall_post_write = false;
    DWORD ignored = 0;
    (void)VirtualProtect(
        reinterpret_cast<void*>(impl_->plans[0].source),
        impl_->plans[0].length,
        PAGE_EXECUTE_READWRITE,
        &ignored);
    protections_restored = false;
    flushed = false;
  }
#endif
  if (!writable || !flushed || !protections_restored) {
    impl_->recovery_required = true;
    return ProductionPatchError::rollback_failed;
  }
  for (const auto& plan : impl_->plans) {
    if (!plan_has_bytes(plan, plan.expected)) {
      impl_->recovery_required = true;
      return ProductionPatchError::rollback_failed;
    }
  }
  if (impl_->dispatch_published) {
    ActiveDispatch* expected = &impl_->active;
    if (!g_active_dispatch.compare_exchange_strong(
            expected, nullptr, std::memory_order_acq_rel)) {
      impl_->recovery_required = true;
      return ProductionPatchError::rollback_failed;
    }
    impl_->dispatch_published = false;
  } else if (g_active_dispatch.load(std::memory_order_acquire) != nullptr) {
    impl_->recovery_required = true;
    return ProductionPatchError::rollback_failed;
  }
  installed_ = false;
  preflighted_ = false;
  primary_image_ = 0;
  session_id_ = 0;
  std::fill(std::begin(gore_as_capture_production_shim_targets),
            std::end(gore_as_capture_production_shim_targets), 0);
  g_preflight_owner = nullptr;
  delete impl_;
  impl_ = nullptr;
  return ProductionPatchError::ok;
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
void ProductionPatchCoordinator::inject_install_post_write_failure_for_test() noexcept {
  if (impl_ != nullptr) impl_->fail_install_post_write = true;
}

void ProductionPatchCoordinator::inject_install_rollback_failure_for_test() noexcept {
  if (impl_ != nullptr) impl_->fail_install_rollback = true;
}

void ProductionPatchCoordinator::inject_uninstall_post_write_failure_for_test() noexcept {
  if (impl_ != nullptr) impl_->fail_uninstall_post_write = true;
}

bool ProductionPatchCoordinator::validate_initial_compile_unwind_for_test() const noexcept {
  if (impl_ == nullptr || !impl_->generated_function_table_registered) return false;
  // Offset 12 begins the tail JMP and is therefore classified as an epilog by the x64
  // unwinder. Probe the final byte of the relocated prolog so both completed PUSH operations
  // must be interpreted from our generated UNWIND_INFO.
  const auto control_pc = static_cast<DWORD64>(impl_->plans[5].trampoline + 11);
  DWORD64 image_base = 0;
  const auto* function = RtlLookupFunctionEntry(control_pc, &image_base, nullptr);
  if (function == nullptr || image_base != reinterpret_cast<DWORD64>(impl_->block) ||
      function->BeginAddress != impl_->generated_unwind_functions[0].BeginAddress ||
      function->UnwindData != impl_->generated_unwind_functions[0].UnwindData) {
    return false;
  }
  constexpr DWORD64 saved_rbx = 0x1111'2222'3333'4444ull;
  constexpr DWORD64 saved_rbp = 0x5555'6666'7777'8888ull;
  constexpr DWORD64 return_address = 0x0000'0001'4000'1000ull;
  alignas(16) std::array<DWORD64, 3> stack{saved_rbx, saved_rbp, return_address};
  CONTEXT context{};
  context.ContextFlags = CONTEXT_FULL;
  context.Rip = control_pc;
  context.Rsp = reinterpret_cast<DWORD64>(stack.data());
  context.Rbx = 1;
  context.Rbp = 2;
  PVOID handler_data = nullptr;
  DWORD64 establisher_frame = 0;
  const auto handler = RtlVirtualUnwind(
      UNW_FLAG_NHANDLER,
      image_base,
      control_pc,
      const_cast<PRUNTIME_FUNCTION>(function),
      &context,
      &handler_data,
      &establisher_frame,
      nullptr);
  if (handler != nullptr || context.Rbx != saved_rbx || context.Rbp != saved_rbp ||
      context.Rip != return_address ||
      context.Rsp != reinterpret_cast<DWORD64>(stack.data() + stack.size())) {
    return false;
  }

  // The stack-home transfer deliberately is not an x64 epilog marker. An asynchronous unwind at
  // its first instruction must still apply both relocated PUSH operations and reach the caller.
  CONTEXT tail{};
  tail.ContextFlags = CONTEXT_FULL;
  tail.Rip = impl_->plans[5].trampoline + 12;
  tail.Rsp = reinterpret_cast<DWORD64>(stack.data());
  tail.Rbx = 1;
  tail.Rbp = 2;
  handler_data = nullptr;
  establisher_frame = 0;
  const auto tail_handler = RtlVirtualUnwind(
      UNW_FLAG_NHANDLER,
      image_base,
      tail.Rip,
      const_cast<PRUNTIME_FUNCTION>(function),
      &tail,
      &handler_data,
      &establisher_frame,
      nullptr);
  if (tail_handler != nullptr || tail.Rip != return_address ||
      tail.Rsp != reinterpret_cast<DWORD64>(stack.data() + stack.size()) ||
      tail.Rbx != saved_rbx || tail.Rbp != saved_rbp) {
    return false;
  }

  for (std::size_t index = 0; index < kProductionRegistrationSiteCount; ++index) {
    const auto& hook = registration::kPinnedRegistrationHooks[index];
    const auto site_id = kProductionBaseSiteCount + index;
    std::uint32_t stack_delta = 0;
    if (!registration_stack_delta(hook, stack_delta)) return false;
    alignas(16) std::array<DWORD64, 512> registration_stack{};
    const auto return_index = static_cast<std::size_t>(stack_delta / sizeof(DWORD64));
    if (return_index >= registration_stack.size()) return false;
    for (std::size_t operation = 0; operation < hook.unwind_operation_count; ++operation) {
      if (hook.unwind[operation].stack_offset >=
          registration_stack.size() * sizeof(DWORD64)) {
        return false;
      }
    }
    const auto registration_return = return_address + index * 16;
    registration_stack[return_index] = registration_return;
    const auto registration_pc = impl_->plans[site_id].trampoline + hook.overwrite_bytes;
    DWORD64 registration_base = 0;
    const auto* registration_function = RtlLookupFunctionEntry(
        registration_pc, &registration_base, nullptr);
    if (registration_function == nullptr ||
        registration_base != reinterpret_cast<DWORD64>(impl_->block) ||
        registration_function->BeginAddress !=
            impl_->generated_unwind_functions[index + 1].BeginAddress) {
      return false;
    }
    CONTEXT registration_context{};
    registration_context.ContextFlags = CONTEXT_FULL;
    registration_context.Rip = registration_pc;
    registration_context.Rsp = reinterpret_cast<DWORD64>(registration_stack.data());
    handler_data = nullptr;
    establisher_frame = 0;
    const auto registration_handler = RtlVirtualUnwind(
        UNW_FLAG_NHANDLER,
        registration_base,
        registration_pc,
        const_cast<PRUNTIME_FUNCTION>(registration_function),
        &registration_context,
        &handler_data,
        &establisher_frame,
        nullptr);
    if (registration_handler != nullptr || registration_context.Rip != registration_return ||
        registration_context.Rsp !=
            reinterpret_cast<DWORD64>(registration_stack.data() + return_index + 1)) {
      return false;
    }
  }
  return true;
}

bool ProductionPatchCoordinator::validate_relay_unwind_for_test() const noexcept {
  if (impl_ == nullptr || !impl_->relay_function_table_registered) return false;
  for (std::size_t index = 0; index < impl_->plans.size(); ++index) {
    const auto& plan = impl_->plans[index];
    if (plan.relay == 0) continue;
    DWORD64 relay_base = 0;
    const auto* relay_function = RtlLookupFunctionEntry(
        static_cast<DWORD64>(plan.relay), &relay_base, nullptr);
    if (relay_function == nullptr || relay_base != primary_image_ ||
        plan.relay < relay_base + relay_function->BeginAddress ||
        plan.relay >= relay_base + relay_function->EndAddress) {
      return false;
    }

    alignas(16) std::array<DWORD64, 1024> stack{};
    for (std::size_t slot = 0; slot < stack.size(); ++slot) {
      stack[slot] = 0x0000'0001'4000'0000ull + slot * 16 + index;
    }
    CONTEXT relay_context{};
    relay_context.ContextFlags = CONTEXT_FULL;
    relay_context.Rip = plan.relay;
    relay_context.Rsp = reinterpret_cast<DWORD64>(stack.data() + 128);
    relay_context.Rax = 1;
    relay_context.Rcx = 2;
    relay_context.Rdx = 3;
    relay_context.Rbx = 4;
    relay_context.Rbp = 5;
    relay_context.Rsi = 6;
    relay_context.Rdi = 7;
    relay_context.R8 = 8;
    relay_context.R9 = 9;
    relay_context.R10 = 10;
    relay_context.R11 = 11;
    relay_context.R12 = 12;
    relay_context.R13 = 13;
    relay_context.R14 = 14;
    relay_context.R15 = 15;

    if (sites_[index].call_rewrite) {
      PVOID handler_data = nullptr;
      DWORD64 establisher_frame = 0;
      const auto expected_return = stack[128];
      const auto handler = RtlVirtualUnwind(
          UNW_FLAG_NHANDLER,
          relay_base,
          plan.relay,
          const_cast<PRUNTIME_FUNCTION>(relay_function),
          &relay_context,
          &handler_data,
          &establisher_frame,
          nullptr);
      if (handler != nullptr || relay_context.Rip != expected_return ||
          relay_context.Rsp != reinterpret_cast<DWORD64>(stack.data() + 129)) {
        return false;
      }
      continue;
    }

    DWORD64 source_base = 0;
    const auto* source_function = RtlLookupFunctionEntry(
        static_cast<DWORD64>(plan.source), &source_base, nullptr);
    if (source_function == nullptr || source_base != primary_image_) return false;
    for (const auto handler_type :
         {DWORD{UNW_FLAG_NHANDLER}, DWORD{UNW_FLAG_EHANDLER}, DWORD{UNW_FLAG_UHANDLER}}) {
      CONTEXT source_context = relay_context;
      source_context.Rip = plan.source;
      CONTEXT compared_relay = relay_context;
      PVOID source_handler_data = nullptr;
      PVOID relay_handler_data = nullptr;
      DWORD64 source_frame = 0;
      DWORD64 relay_frame = 0;
      const auto source_handler = RtlVirtualUnwind(
          handler_type,
          source_base,
          plan.source,
          const_cast<PRUNTIME_FUNCTION>(source_function),
          &source_context,
          &source_handler_data,
          &source_frame,
          nullptr);
      const auto relay_handler = RtlVirtualUnwind(
          handler_type,
          relay_base,
          plan.relay,
          const_cast<PRUNTIME_FUNCTION>(relay_function),
          &compared_relay,
          &relay_handler_data,
          &relay_frame,
          nullptr);
      if (source_handler != relay_handler || source_handler_data != relay_handler_data ||
          source_frame != relay_frame ||
          std::memcmp(&source_context, &compared_relay, sizeof(CONTEXT)) != 0) {
        return false;
      }
    }
  }
  return true;
}

bool ProductionPatchCoordinator::validate_unsafe_ranges_for_test() const noexcept {
  if (impl_ == nullptr) return false;
  std::array<std::pair<std::uintptr_t, std::uintptr_t>, kPatchRangeCount> ranges{};
  if (!patch_ranges(*impl_, ranges)) return false;
  const auto image = impl_->plans[0].source - impl_->plans[0].patch_rva;
  return std::all_of(kUnsafeInstallRanges.begin(), kUnsafeInstallRanges.end(), [&](const auto& unsafe) {
    const std::pair expected{image + unsafe.begin_rva, image + unsafe.end_rva};
    return std::find(ranges.begin(), ranges.end(), expected) != ranges.end();
  });
}
#endif

ProductionPatchError ProductionPatchCoordinator::prepare_unload() const noexcept {
  if (installed_ ||
      (impl_ != nullptr && (impl_->active.active_return_frames.load() != 0 ||
                            impl_->active.active_dispatches.load() != 0))) {
    return ProductionPatchError::active_return_frames;
  }
  return ProductionPatchError::ok;
}

extern "C" void __cdecl gore_as_capture_production_shim_before(
    ProductionMachineFrame* const frame,
    const std::uint32_t site_id) noexcept {
  auto* const active = g_active_dispatch.load(std::memory_order_acquire);
  if (active == nullptr || frame == nullptr || site_id >= kProductionSiteCount) return;
  DispatchLease lease(active);
  if (g_active_dispatch.load(std::memory_order_acquire) != active) return;
  if (!active->observer.dispatch(
          active->observer.context, site_id, ProductionShimPhase::before, *frame)) {
    active->observer_failed.store(true, std::memory_order_release);
    return;
  }
  if (!requires_return_substitution(site_id)) return;
  if (g_return_stack.depth == g_return_stack.frames.size() ||
      !readable_writable_pointer(frame->rsp, sizeof(std::uintptr_t))) {
    active->observer_failed.store(true, std::memory_order_release);
    return;
  }
  std::uintptr_t original_return = 0;
  std::memcpy(&original_return, reinterpret_cast<const void*>(frame->rsp), sizeof(original_return));
  if (original_return == 0) {
    active->observer_failed.store(true, std::memory_order_release);
    return;
  }
  g_return_stack.frames[g_return_stack.depth++] =
      ReturnFrame{site_id, frame->rsp, original_return, active};
  const auto replacement =
      reinterpret_cast<std::uintptr_t>(gore_as_capture_production_return);
  std::memcpy(reinterpret_cast<void*>(frame->rsp), &replacement, sizeof(replacement));
  active->active_return_frames.fetch_add(1, std::memory_order_acq_rel);
}

extern "C" void __cdecl gore_as_capture_production_shim_after(
    ProductionMachineFrame* const frame) noexcept {
  if (frame == nullptr || g_return_stack.depth == 0) return;
  const ReturnFrame pending = g_return_stack.frames[--g_return_stack.depth];
  DispatchLease lease(pending.owner);
  if (pending.owner == nullptr || frame->rsp < sizeof(std::uintptr_t) ||
      pending.substituted_slot != frame->rsp - sizeof(std::uintptr_t) ||
      !readable_writable_pointer(pending.substituted_slot, sizeof(std::uintptr_t))) {
    if (pending.owner != nullptr) {
      pending.owner->observer_failed.store(true, std::memory_order_release);
      pending.owner->active_return_frames.fetch_sub(1, std::memory_order_acq_rel);
    }
    return;
  }
  std::memcpy(
      reinterpret_cast<void*>(pending.substituted_slot),
      &pending.original_return,
      sizeof(pending.original_return));
  if (!pending.owner->observer.dispatch(
          pending.owner->observer.context,
          pending.site_id,
          ProductionShimPhase::after,
          *frame)) {
    pending.owner->observer_failed.store(true, std::memory_order_release);
  }
  pending.owner->active_return_frames.fetch_sub(1, std::memory_order_acq_rel);
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

bool fixture_dispatch(
    void* const context,
    const std::uint32_t site_id,
    const ProductionShimPhase phase,
    ProductionMachineFrame&) noexcept {
  if (context == nullptr || site_id >= kProductionSiteCount) return false;
  auto& order = *static_cast<std::vector<std::uint32_t>*>(context);
  try {
    order.push_back(site_id * 2 + (phase == ProductionShimPhase::after ? 1u : 0u));
    return true;
  } catch (...) {
    return false;
  }
}

struct StallDispatch final {
  HANDLE entered{};
  HANDLE release{};
};

bool stall_dispatch(
    void* const context,
    const std::uint32_t,
    const ProductionShimPhase,
    ProductionMachineFrame&) noexcept {
  auto& stall = *static_cast<StallDispatch*>(context);
  return SetEvent(stall.entered) != FALSE &&
         WaitForSingleObject(stall.release, 5'000) == WAIT_OBJECT_0;
}

bool selftest_return_stack() {
  ActiveDispatch active{};
  std::vector<std::uint32_t> order;
  active.observer = ProductionShimObserver{&order, fixture_dispatch};
  ActiveDispatch* expected = nullptr;
  if (!g_active_dispatch.compare_exchange_strong(expected, &active)) return false;
  std::array<std::uintptr_t, 4> stack{
      0x1111'1111u, 0x2222'2222u, 0x3333'3333u, 0x4444'4444u};
  ProductionMachineFrame outer{};
  ProductionMachineFrame inner{};
  outer.rsp = reinterpret_cast<std::uintptr_t>(&stack[0]);
  inner.rsp = reinterpret_cast<std::uintptr_t>(&stack[1]);
  gore_as_capture_production_shim_before(&outer, 9);
  gore_as_capture_production_shim_before(&inner, 10);
  inner.rsp += sizeof(std::uintptr_t);
  gore_as_capture_production_shim_after(&inner);
  outer.rsp += sizeof(std::uintptr_t);
  gore_as_capture_production_shim_after(&outer);
  expected = &active;
  const bool removed = g_active_dispatch.compare_exchange_strong(expected, nullptr);
  return removed && !active.observer_failed.load() && active.active_return_frames.load() == 0 &&
         stack[0] == 0x1111'1111u && stack[1] == 0x2222'2222u &&
         order == std::vector<std::uint32_t>({18, 20, 21, 19});
}

bool selftest_unwind_entries() noexcept {
  for (const auto entry : kShimEntries) {
    DWORD64 image_base = 0;
    if (RtlLookupFunctionEntry(
            reinterpret_cast<DWORD64>(entry), &image_base, nullptr) == nullptr ||
        image_base == 0) {
      return false;
    }
  }
  DWORD64 image_base = 0;
  return RtlLookupFunctionEntry(
             reinterpret_cast<DWORD64>(gore_as_capture_production_return),
             &image_base,
             nullptr) != nullptr &&
         image_base != 0;
}

bool selftest_site_manifest() noexcept {
  std::array<bool, kProductionSiteCount> seen{};
  for (std::size_t index = 0; index < kProductionSiteCount; ++index) {
    if (kShimEntries[index] == nullptr || seen[index]) return false;
    seen[index] = true;
  }
  std::size_t returns = 0;
  for (std::uint32_t index = 0; index < kProductionSiteCount; ++index) {
    returns += requires_return_substitution(index) ? 1u : 0u;
  }
  return std::all_of(seen.begin(), seen.end(), [](const bool value) { return value; }) &&
         returns == 21;
}

class SyntheticPinnedImage final {
 public:
  static constexpr std::uint32_t kFixtureUnwindRva = 0x1000;
  static constexpr std::uint32_t kInlineFunctionBegin = 0x04685600;
  static constexpr std::uint32_t kInlineFunctionEnd = 0x04685c60;

  SyntheticPinnedImage() noexcept
      : image_(static_cast<std::byte*>(VirtualAlloc(
            nullptr, kPeSizeOfImage, MEM_RESERVE, PAGE_NOACCESS))) {
    if (image_ == nullptr) return;
    SYSTEM_INFO system{};
    GetSystemInfo(&system);
    page_bytes_ = system.dwPageSize;
    if (page_bytes_ == 0) return;
    for (const auto& site : kPinnedInstructionSpans) {
      if (!commit(site.patch_anchor_rva, site.byte_count)) return;
      std::memcpy(image_ + site.patch_anchor_rva, site.expected.data(), site.byte_count);
    }
    for (const auto& site : registration::kPinnedRegistrationHooks) {
      if (!commit(site.function_rva, site.overwrite_bytes)) return;
      std::memcpy(image_ + site.function_rva, site.expected.data(), site.overwrite_bytes);
    }
    for (const auto& site : frontend_target_layout::callback_callsites) {
      if (!commit(site.call_rva, site.expected_call.size())) return;
      std::memcpy(image_ + site.call_rva, site.expected_call.data(), site.expected_call.size());
    }
    if (!commit(kFixtureUnwindRva, 16)) return;
    image_[kFixtureUnwindRva] = std::byte{1};
    constexpr std::array<std::byte, 8> inline_unwind{
        std::byte{1}, std::byte{4}, std::byte{2}, std::byte{0},
        std::byte{4}, std::byte{0x32}, std::byte{1}, std::byte{0x30}};
    std::memcpy(
        image_ + kFixtureUnwindRva + 8, inline_unwind.data(), inline_unwind.size());
    source_functions_ = {{
        {kPinnedInstructionSpans[5].patch_anchor_rva,
         kPinnedInstructionSpans[5].patch_anchor_rva + 0x100,
         kFixtureUnwindRva},
        {kInlineFunctionBegin, kInlineFunctionEnd, kFixtureUnwindRva + 8},
        {kPinnedInstructionSpans[0].patch_anchor_rva,
         kPinnedInstructionSpans[0].patch_anchor_rva + 0x100,
         kFixtureUnwindRva},
        {kPinnedInstructionSpans[4].patch_anchor_rva,
         kPinnedInstructionSpans[4].patch_anchor_rva + 0x100,
         kFixtureUnwindRva},
        {kPinnedInstructionSpans[3].patch_anchor_rva,
         kPinnedInstructionSpans[3].patch_anchor_rva + 0x100,
         kFixtureUnwindRva},
    }};
    std::sort(
        source_functions_.begin(), source_functions_.end(),
        [](const RUNTIME_FUNCTION& left, const RUNTIME_FUNCTION& right) {
          return left.BeginAddress < right.BeginAddress;
        });
    for (const auto page : committed_pages_) {
      DWORD ignored = 0;
      if (VirtualProtect(image_ + page, page_bytes_, PAGE_EXECUTE_READ, &ignored) == FALSE) {
        return;
      }
    }
    if (RtlAddFunctionTable(
            source_functions_.data(),
            static_cast<DWORD>(source_functions_.size()),
            reinterpret_cast<DWORD64>(image_)) == FALSE) {
      return;
    }
    function_table_registered_ = true;
    ready_ = true;
  }

  ~SyntheticPinnedImage() {
    if (function_table_registered_) {
      (void)RtlDeleteFunctionTable(source_functions_.data());
    }
    if (image_ != nullptr) (void)VirtualFree(image_, 0, MEM_RELEASE);
  }
  SyntheticPinnedImage(const SyntheticPinnedImage&) = delete;
  SyntheticPinnedImage& operator=(const SyntheticPinnedImage&) = delete;
  std::uintptr_t address() const noexcept {
    return reinterpret_cast<std::uintptr_t>(image_);
  }
  bool ready() const noexcept { return ready_; }
  bool originals_match() const noexcept {
    if (!ready_) return false;
    for (const auto& site : kPinnedInstructionSpans) {
      if (std::memcmp(
              image_ + site.patch_anchor_rva, site.expected.data(), site.byte_count) != 0) {
        return false;
      }
    }
    for (const auto& site : registration::kPinnedRegistrationHooks) {
      if (std::memcmp(
              image_ + site.function_rva, site.expected.data(), site.overwrite_bytes) != 0) {
        return false;
      }
    }
    for (const auto& site : frontend_target_layout::callback_callsites) {
      if (std::memcmp(
              image_ + site.call_rva, site.expected_call.data(), site.expected_call.size()) != 0) {
        return false;
      }
    }
    return true;
  }
  bool protections_execute_read() const noexcept {
    if (!ready_) return false;
    for (const auto page : committed_pages_) {
      MEMORY_BASIC_INFORMATION region{};
      if (VirtualQuery(image_ + page, &region, sizeof(region)) != sizeof(region) ||
          region.Protect != PAGE_EXECUTE_READ) {
        return false;
      }
    }
    return true;
  }

 private:
  bool commit(const std::uint32_t rva, const std::size_t bytes) {
    if (rva >= kPeSizeOfImage || bytes > kPeSizeOfImage - rva) return false;
    const auto first = static_cast<std::uint32_t>(rva - rva % page_bytes_);
    const auto last = static_cast<std::uint32_t>(
        (rva + bytes - 1) - (rva + bytes - 1) % page_bytes_);
    for (std::uint32_t page = first;; page += page_bytes_) {
      if (std::find(committed_pages_.begin(), committed_pages_.end(), page) ==
          committed_pages_.end()) {
        if (VirtualAlloc(
                image_ + page, page_bytes_, MEM_COMMIT, PAGE_READWRITE) != image_ + page) {
          return false;
        }
        committed_pages_.push_back(page);
      }
      if (page == last) break;
      if (page > std::numeric_limits<std::uint32_t>::max() - page_bytes_) return false;
    }
    return true;
  }

  std::byte* image_{};
  std::uint32_t page_bytes_{};
  std::array<RUNTIME_FUNCTION, 5> source_functions_{};
  std::vector<std::uint32_t> committed_pages_;
  bool function_table_registered_{};
  bool ready_{};
};

bool selftest_full_transaction() {
  SyntheticPinnedImage image;
  if (!image.ready() || !image.originals_match() || !image.protections_execute_read()) {
    return false;
  }
  std::vector<std::uint32_t> events;
  {
    const HANDLE entered = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    const HANDLE release = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (entered == nullptr || release == nullptr) {
      if (entered != nullptr) (void)CloseHandle(entered);
      if (release != nullptr) (void)CloseHandle(release);
      return false;
    }
    StallDispatch stall{entered, release};
    ProductionPatchCoordinator coordinator;
    if (coordinator.preflight(
            image.address(), 1, ProductionShimObserver{&stall, stall_dispatch}) !=
            ProductionPatchError::ok ||
        coordinator.sites().size() != kProductionSiteCount ||
        !coordinator.validate_initial_compile_unwind_for_test() ||
        !coordinator.validate_relay_unwind_for_test() ||
        !coordinator.validate_unsafe_ranges_for_test() ||
        coordinator.install() != ProductionPatchError::ok || !coordinator.installed() ||
        coordinator.prepare_unload() != ProductionPatchError::active_return_frames) {
      (void)CloseHandle(release);
      (void)CloseHandle(entered);
      return false;
    }
    std::thread dispatch_thread([] {
      ProductionMachineFrame frame{};
      gore_as_capture_production_shim_before(&frame, 0);
    });
    if (WaitForSingleObject(entered, 5'000) != WAIT_OBJECT_0 ||
        coordinator.uninstall() != ProductionPatchError::active_return_frames) {
      (void)SetEvent(release);
      dispatch_thread.join();
      (void)CloseHandle(release);
      (void)CloseHandle(entered);
      return false;
    }
    (void)SetEvent(release);
    dispatch_thread.join();
    (void)CloseHandle(release);
    (void)CloseHandle(entered);
    std::uint32_t wrong_thread = 0;
    std::thread other([&] {
      wrong_thread = static_cast<std::uint32_t>(coordinator.uninstall());
    });
    other.join();
    if (wrong_thread != static_cast<std::uint32_t>(ProductionPatchError::wrong_thread) ||
        !coordinator.installed() || coordinator.uninstall() != ProductionPatchError::ok ||
        coordinator.installed() || !image.originals_match() ||
        !image.protections_execute_read() ||
        coordinator.prepare_unload() != ProductionPatchError::ok) {
      return false;
    }
  }

  // A recoverable post-write failure that proves exact rollback does not retain ownership.
  {
    ProductionPatchCoordinator rolled_back;
    if (rolled_back.preflight(
            image.address(), 2, ProductionShimObserver{&events, fixture_dispatch}) !=
            ProductionPatchError::ok) {
      return false;
    }
    rolled_back.inject_install_post_write_failure_for_test();
    if (rolled_back.install() != ProductionPatchError::patch_failed ||
        rolled_back.installed() || !image.originals_match() ||
        !image.protections_execute_read()) {
      return false;
    }
  }

  // If exact rollback cannot be proved, live replacements, relays and unwind metadata remain
  // owned until an owner-thread recovery uninstall succeeds.
  {
    ProductionPatchCoordinator recovery;
    if (recovery.preflight(
            image.address(), 3, ProductionShimObserver{&events, fixture_dispatch}) !=
            ProductionPatchError::ok) {
      return false;
    }
    recovery.inject_install_post_write_failure_for_test();
    recovery.inject_install_rollback_failure_for_test();
    if (recovery.install() != ProductionPatchError::rollback_failed ||
        !recovery.installed() ||
        recovery.prepare_unload() != ProductionPatchError::active_return_frames ||
        recovery.uninstall() != ProductionPatchError::ok || recovery.installed() ||
        !image.originals_match() || !image.protections_execute_read()) {
      return false;
    }
  }

  // A failed uninstall is retryable and must retain all dispatch/trampoline ownership.
  {
    ProductionPatchCoordinator retry;
    if (retry.preflight(
            image.address(), 4, ProductionShimObserver{&events, fixture_dispatch}) !=
            ProductionPatchError::ok ||
        retry.install() != ProductionPatchError::ok) {
      return false;
    }
    retry.inject_uninstall_post_write_failure_for_test();
    if (retry.uninstall() != ProductionPatchError::rollback_failed || !retry.installed() ||
        retry.prepare_unload() != ProductionPatchError::active_return_frames ||
        retry.uninstall() != ProductionPatchError::ok || retry.installed() ||
        !image.originals_match() || !image.protections_execute_read()) {
      return false;
    }
  }

  // A worker whose instruction pointer is inside a target lifecycle range blocks the entire
  // transaction before any source write. The test-only routine signals entry and then polls a
  // release byte entirely within InitialCompile's pinned unsafe interval.
  {
    constexpr std::uint32_t stall_rva = kUnsafeInstallRanges[0].begin_rva + 0x300;
    const auto stall_address = image.address() + stall_rva;
    const HANDLE entered = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (entered == nullptr) return false;
    std::atomic<std::uint8_t> release_flag{};
    std::array<std::byte, 46> code{};
    std::size_t cursor = 0;
    const auto append = [&](const auto& bytes) {
      std::copy(bytes.begin(), bytes.end(), code.begin() + cursor);
      cursor += bytes.size();
    };
    constexpr std::array<std::byte, 2> mov_rcx{std::byte{0x48}, std::byte{0xb9}};
    constexpr std::array<std::byte, 2> mov_rax{std::byte{0x48}, std::byte{0xb8}};
    constexpr std::array<std::byte, 4> stack_down{
        std::byte{0x48}, std::byte{0x83}, std::byte{0xec}, std::byte{0x28}};
    constexpr std::array<std::byte, 2> call_rax{std::byte{0xff}, std::byte{0xd0}};
    constexpr std::array<std::byte, 4> stack_up{
        std::byte{0x48}, std::byte{0x83}, std::byte{0xc4}, std::byte{0x28}};
    constexpr std::array<std::byte, 3> compare_zero{
        std::byte{0x80}, std::byte{0x38}, std::byte{0x00}};
    constexpr std::array<std::byte, 3> loop_or_return{
        std::byte{0x74}, std::byte{0xfb}, std::byte{0xc3}};
    append(mov_rcx);
    const auto entered_value = reinterpret_cast<std::uintptr_t>(entered);
    std::memcpy(code.data() + cursor, &entered_value, sizeof(entered_value));
    cursor += sizeof(entered_value);
    append(mov_rax);
    const auto set_event = reinterpret_cast<std::uintptr_t>(&SetEvent);
    std::memcpy(code.data() + cursor, &set_event, sizeof(set_event));
    cursor += sizeof(set_event);
    append(stack_down);
    append(call_rax);
    append(stack_up);
    append(mov_rax);
    const auto release_address = reinterpret_cast<std::uintptr_t>(&release_flag);
    std::memcpy(code.data() + cursor, &release_address, sizeof(release_address));
    cursor += sizeof(release_address);
    const auto loop_address = stall_address + cursor;
    append(compare_zero);
    append(loop_or_return);
    if (cursor != code.size()) {
      (void)CloseHandle(entered);
      return false;
    }
    DWORD old = 0;
    if (VirtualProtect(
            reinterpret_cast<void*>(stall_address),
            code.size(),
            PAGE_EXECUTE_READWRITE,
            &old) == FALSE) {
      (void)CloseHandle(entered);
      return false;
    }
    std::memcpy(reinterpret_cast<void*>(stall_address), code.data(), code.size());
    const bool code_ready = FlushInstructionCache(
                                GetCurrentProcess(),
                                reinterpret_cast<void*>(stall_address),
                                code.size()) != FALSE;
    DWORD ignored = 0;
    const bool code_protected = VirtualProtect(
                                    reinterpret_cast<void*>(stall_address),
                                    code.size(),
                                    old,
                                    &ignored) != FALSE;
    if (!code_ready || !code_protected) {
      (void)CloseHandle(entered);
      return false;
    }
    ProductionPatchCoordinator rip_guard;
    if (rip_guard.preflight(
            image.address(), 6, ProductionShimObserver{&events, fixture_dispatch}) !=
        ProductionPatchError::ok) {
      (void)CloseHandle(entered);
      return false;
    }
    std::thread worker([stall_address] {
      reinterpret_cast<void (*)()>(stall_address)();
    });
    bool observed_loop = WaitForSingleObject(entered, 5'000) == WAIT_OBJECT_0;
    bool at_loop = false;
    for (std::uint32_t attempt = 0; observed_loop && attempt < 100 && !at_loop; ++attempt) {
      if (SuspendThread(worker.native_handle()) == std::numeric_limits<DWORD>::max()) {
        observed_loop = false;
        break;
      }
      CONTEXT context{};
      context.ContextFlags = CONTEXT_CONTROL;
      observed_loop = GetThreadContext(worker.native_handle(), &context) != FALSE;
      at_loop = observed_loop && context.Rip >= loop_address &&
                context.Rip < loop_address + compare_zero.size() + 2;
      (void)ResumeThread(worker.native_handle());
      if (!at_loop) (void)SwitchToThread();
    }
    const auto guarded = at_loop ? rip_guard.install()
                                 : ProductionPatchError::invalid_state;
    release_flag.store(1, std::memory_order_release);
    worker.join();
    (void)CloseHandle(entered);
    if (guarded != ProductionPatchError::thread_in_patch_range || rip_guard.installed() ||
        !image.originals_match() || !image.protections_execute_read()) {
      return false;
    }
  }

  // A drifted 26-site preflight is a pure refusal: no earlier site is ever modified.
  const auto drift = image.address() + frontend_target_layout::class_analyze_call_rva;
  DWORD old = 0;
  if (VirtualProtect(reinterpret_cast<void*>(drift), 5, PAGE_EXECUTE_READWRITE, &old) == FALSE) {
    return false;
  }
  reinterpret_cast<std::byte*>(drift)[2] ^= std::byte{1};
  DWORD ignored = 0;
  if (VirtualProtect(reinterpret_cast<void*>(drift), 5, old, &ignored) == FALSE) return false;
  ProductionPatchCoordinator rejected;
  const auto refusal = rejected.preflight(
      image.address(), 7, ProductionShimObserver{&events, fixture_dispatch});
  if (VirtualProtect(reinterpret_cast<void*>(drift), 5, PAGE_EXECUTE_READWRITE, &old) == FALSE) {
    return false;
  }
  reinterpret_cast<std::byte*>(drift)[2] ^= std::byte{1};
  (void)VirtualProtect(reinterpret_cast<void*>(drift), 5, old, &ignored);
  return refusal == ProductionPatchError::target_drift && image.originals_match();
}

}  // namespace

std::uint32_t production_observer_shims_selftest_stages_v1() noexcept {
  std::uint32_t stages = 0;
  try {
    stages |= selftest_site_manifest() ? 1u << 0 : 0;
    stages |= selftest_unwind_entries() ? 1u << 1 : 0;
    std::uint32_t machine_state_result = 0;
    std::thread machine_state_fixture([&] {
      machine_state_result = gore_as_capture_production_shim_state_selftest();
    });
    machine_state_fixture.join();
    stages |= machine_state_result == 1 ? 1u << 2 : 0;
    stages |= selftest_return_stack() ? 1u << 3 : 0;
    stages |= selftest_full_transaction() ? 1u << 4 : 0;
  } catch (...) {
  }
  return stages;
}

bool production_observer_shims_selftest_v1() noexcept {
  return production_observer_shims_selftest_stages_v1() == 0x1f;
}

#endif

}  // namespace gore_as_capture::v1::instrumentation
