#define GORE_AS_CAPTURE_BRIDGE_BUILD
#include "gore_as_capture/instrumentation.h"

#include "gore_as_capture/format.hpp"
#include "gore_as_capture/instrumentation.hpp"
#include "production_capture_phase_machine.hpp"
#include "production_capture_dispatcher.hpp"
#include "production_observer_shims.hpp"
#include "target_capture_serializer.hpp"
#include "target_final_state.hpp"
#include "target_frontend_observer.hpp"
#include "target_frontend_raw_materializer.hpp"
#include "target_frontend_snapshot_builder.hpp"
#include "target_layout.hpp"
#include "target_registration_observer.hpp"
#include "target_snapshot.hpp"
#include "target_type_usage.hpp"

#include <windows.h>
#include <tlhelp32.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <mutex>
#include <new>
#include <span>
#include <thread>
#include <type_traits>
#include <utility>
#include <vector>

namespace {

namespace target = gore_as_capture::v1;
namespace adapter = gore_as_capture::v1::instrumentation;
namespace registration = gore_as_capture::v1::instrumentation::registration;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
constexpr std::uint32_t kTestFixtureOnly = 1;
#else
constexpr std::uint32_t kTestFixtureOnly = 0;
#endif

static_assert(sizeof(gore_as_capture_instrumentation_contract_v1) ==
              GORE_AS_CAPTURE_INSTRUMENTATION_CONTRACT_BYTES_V1);
static_assert(sizeof(gore_as_capture_instrumentation_site_contract_v1) ==
              GORE_AS_CAPTURE_INSTRUMENTATION_SITE_CONTRACT_BYTES_V1);
static_assert(sizeof(gore_as_capture_registration_hook_set_v1) ==
              GORE_AS_CAPTURE_REGISTRATION_HOOK_SET_BYTES_V1);
static_assert(sizeof(gore_as_capture_registration_site_contract_v1) ==
              GORE_AS_CAPTURE_REGISTRATION_SITE_CONTRACT_BYTES_V1);
static_assert(sizeof(gore_as_capture_instrumentation_selftest_v1) ==
              GORE_AS_CAPTURE_INSTRUMENTATION_SELFTEST_BYTES_V1);

struct InstrumentationState final {
  std::mutex mutex;
  std::unique_ptr<adapter::ProductionCaptureCoordinator> coordinator;
  bool installed{};
  std::uint64_t session_id{};
  DWORD owner_thread{};
};

InstrumentationState& instrumentation_state() {
  static InstrumentationState state;
  return state;
}

std::uint32_t instrumentation_error(
    const adapter::ProductionCaptureCoordinatorError error) noexcept {
  switch (error) {
    case adapter::ProductionCaptureCoordinatorError::ok:
      return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
    case adapter::ProductionCaptureCoordinatorError::wrong_thread:
      return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1;
    case adapter::ProductionCaptureCoordinatorError::target_drift:
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    case adapter::ProductionCaptureCoordinatorError::patch_failure:
      return GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
    case adapter::ProductionCaptureCoordinatorError::recovery_required:
      return GORE_AS_CAPTURE_INSTRUMENTATION_ROLLBACK_FAILED_V1;
    case adapter::ProductionCaptureCoordinatorError::semantic_failure:
      return GORE_AS_CAPTURE_INSTRUMENTATION_UNRESOLVED_SEMANTICS_V1;
    case adapter::ProductionCaptureCoordinatorError::invalid_state:
    case adapter::ProductionCaptureCoordinatorError::terminal_failure:
      return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
  return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
}

bool is_readable_protection(const DWORD protection) noexcept {
  switch (protection & 0xffu) {
    case PAGE_READONLY:
    case PAGE_READWRITE:
    case PAGE_WRITECOPY:
    case PAGE_EXECUTE_READ:
    case PAGE_EXECUTE_READWRITE:
    case PAGE_EXECUTE_WRITECOPY:
      return true;
    default:
      return false;
  }
}

bool readable_range(const std::uintptr_t first, const std::size_t size) noexcept {
  if (first == 0 || size == 0 || first > std::numeric_limits<std::uintptr_t>::max() - size) {
    return false;
  }
  std::uintptr_t cursor = first;
  const std::uintptr_t end = first + size;
  while (cursor < end) {
    MEMORY_BASIC_INFORMATION region{};
    if (VirtualQuery(reinterpret_cast<const void*>(cursor), &region, sizeof(region)) !=
            sizeof(region) ||
        region.State != MEM_COMMIT || (region.Protect & PAGE_GUARD) != 0 ||
        !is_readable_protection(region.Protect)) {
      return false;
    }
    const auto base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
    if (base > std::numeric_limits<std::uintptr_t>::max() - region.RegionSize) {
      return false;
    }
    const std::uintptr_t next = base + region.RegionSize;
    if (next <= cursor) {
      return false;
    }
    cursor = std::min(next, end);
  }
  return true;
}

template <typename Integer>
bool add_signed_displacement(
    const std::uintptr_t instruction_end,
    const Integer displacement,
    std::uintptr_t& target_out) noexcept {
  static_assert(std::is_signed_v<Integer>);
  const auto wide_end = static_cast<std::int64_t>(instruction_end);
  const auto wide_displacement = static_cast<std::int64_t>(displacement);
  if ((wide_displacement > 0 &&
       wide_end > std::numeric_limits<std::int64_t>::max() - wide_displacement) ||
      (wide_displacement < 0 &&
       wide_end < std::numeric_limits<std::int64_t>::min() - wide_displacement)) {
    return false;
  }
  const auto target = wide_end + wide_displacement;
  if (target < 0) return false;
  target_out = static_cast<std::uintptr_t>(target);
  return true;
}

bool relative_displacement(
    const std::uintptr_t instruction_end,
    const std::uintptr_t target,
    std::int32_t& displacement_out) noexcept {
  const auto delta = static_cast<std::int64_t>(target) -
                     static_cast<std::int64_t>(instruction_end);
  if (delta < std::numeric_limits<std::int32_t>::min() ||
      delta > std::numeric_limits<std::int32_t>::max()) {
    return false;
  }
  displacement_out = static_cast<std::int32_t>(delta);
  return true;
}

struct TrampolinePlan final {
  std::array<std::byte, 64> bytes{};
  std::uint8_t byte_count{};
  bool contains_call{};
};

bool append_indirect_jump(
    TrampolinePlan& plan,
    const std::uintptr_t target) noexcept {
  constexpr std::array<std::byte, 6> instruction{
      std::byte{0xff}, std::byte{0x25}, std::byte{0x00},
      std::byte{0x00}, std::byte{0x00}, std::byte{0x00}};
  if (plan.byte_count > plan.bytes.size() - instruction.size() - sizeof(target)) return false;
  std::memcpy(plan.bytes.data() + plan.byte_count, instruction.data(), instruction.size());
  plan.byte_count = static_cast<std::uint8_t>(plan.byte_count + instruction.size());
  std::memcpy(plan.bytes.data() + plan.byte_count, &target, sizeof(target));
  plan.byte_count = static_cast<std::uint8_t>(plan.byte_count + sizeof(target));
  return true;
}

bool relocate_relative_field(
    TrampolinePlan& plan,
    const std::uintptr_t original,
    const std::uintptr_t relocated,
    const std::size_t displacement_offset,
    const std::size_t instruction_end_offset) noexcept {
  std::int32_t old_displacement = 0;
  std::memcpy(
      &old_displacement, plan.bytes.data() + displacement_offset, sizeof(old_displacement));
  std::uintptr_t target = 0;
  if (!add_signed_displacement(
          original + instruction_end_offset, old_displacement, target)) {
    return false;
  }
  std::int32_t new_displacement = 0;
  if (!relative_displacement(
          relocated + instruction_end_offset, target, new_displacement)) {
    return false;
  }
  std::memcpy(
      plan.bytes.data() + displacement_offset, &new_displacement, sizeof(new_displacement));
  return true;
}

bool build_trampoline_plan(
    const std::size_t index,
    const std::uintptr_t original,
    const std::uintptr_t relocated,
    TrampolinePlan& plan) noexcept {
  if (index >= adapter::kPinnedInstructionSpans.size()) return false;
  const auto& site = adapter::kPinnedInstructionSpans[index];
  plan = {};
  std::memcpy(plan.bytes.data(), site.expected.data(), site.byte_count);
  plan.byte_count = site.byte_count;
  switch (index) {
    case 0:  // ja rel32 in SetEngineProperty's entry span.
      if (!relocate_relative_field(plan, original, relocated, 7, 11)) return false;
      break;
    case 1:  // Indirect callback call; the trampoline is an unwindable leaf caller.
      plan.contains_call = true;
      break;
    case 4:  // RIP-relative load of the active FStaticJITCompiledInfo pointer.
      if (!relocate_relative_field(plan, original, relocated, 3, 7)) return false;
      break;
    case 6:
    case 7:  // Direct rel32 calls at the two frontend boundaries.
      plan.contains_call = true;
      if (!relocate_relative_field(plan, original, relocated, 1, 5)) return false;
      break;
    default:
      break;
  }
  if (index == 4) {
    // The complete getter, including RET, is displaced; appending a continuation is wrong.
    return true;
  }
  return append_indirect_jump(plan, original + site.byte_count);
}

struct TypedBindMetadata final {
  std::int32_t bind_order{};
  std::uint32_t callback_rva{};
  bool final_callback{};
  std::uintptr_t engine_capability{};
};

struct TypedFrontendMetadata final {
  std::uintptr_t manager_capability{};
  std::uintptr_t engine_capability{};
  std::uintptr_t boundary_object_capability{};
  std::uint32_t item_count{};
  bool initial_compile_succeeded{};
};

struct RegistrationEntryFrame final {
  std::uintptr_t rcx{};
  std::uintptr_t rdx{};
  std::uintptr_t r8{};
  std::uintptr_t r9{};
  std::uintptr_t original_rsp{};
};

using RegistrationArgumentProjection = adapter::RawRegistrationArgument;
using ExtractedRegistrationEntry = adapter::RawRegistrationEntry;

bool valid_utf8(const std::span<const char> bytes) noexcept {
  std::size_t cursor = 0;
  while (cursor < bytes.size()) {
    const auto first = static_cast<std::uint8_t>(bytes[cursor]);
    if (first <= 0x7f) {
      ++cursor;
      continue;
    }
    std::size_t trailing = 0;
    std::uint32_t codepoint = 0;
    std::uint32_t minimum = 0;
    if ((first & 0xe0u) == 0xc0u) {
      trailing = 1;
      codepoint = first & 0x1fu;
      minimum = 0x80;
    } else if ((first & 0xf0u) == 0xe0u) {
      trailing = 2;
      codepoint = first & 0x0fu;
      minimum = 0x800;
    } else if ((first & 0xf8u) == 0xf0u) {
      trailing = 3;
      codepoint = first & 0x07u;
      minimum = 0x10000;
    } else {
      return false;
    }
    if (trailing > bytes.size() - cursor - 1) return false;
    for (std::size_t index = 1; index <= trailing; ++index) {
      const auto next = static_cast<std::uint8_t>(bytes[cursor + index]);
      if ((next & 0xc0u) != 0x80u) return false;
      codepoint = (codepoint << 6) | (next & 0x3fu);
    }
    if (codepoint < minimum || codepoint > 0x10ffff ||
        (codepoint >= 0xd800 && codepoint <= 0xdfff)) {
      return false;
    }
    cursor += trailing + 1;
  }
  return true;
}

bool copy_bounded_utf8(
    const std::uintptr_t source,
    RegistrationArgumentProjection& projection) noexcept {
  if (source == 0) return false;
  constexpr std::size_t kMaximumTextBytes = 1024;
  if (source > std::numeric_limits<std::uintptr_t>::max() - kMaximumTextBytes) return false;
  for (std::size_t index = 0; index <= kMaximumTextBytes; ++index) {
    if (!readable_range(source + index, 1)) return false;
    char value = 0;
    std::memcpy(&value, reinterpret_cast<const void*>(source + index), 1);
    if (value == '\0') {
      if (!valid_utf8(std::span(projection.text.data(), index))) return false;
      projection.text_bytes = static_cast<std::uint32_t>(index);
      projection.text[index] = '\0';
      return true;
    }
    if (index == kMaximumTextBytes) return false;
    projection.text[index] = value;
  }
  return false;
}

bool registration_argument_value(
    const RegistrationEntryFrame& frame,
    const std::uint8_t source,
    std::uintptr_t& value_out) noexcept {
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1:
      value_out = frame.rdx;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1:
      value_out = frame.r8;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1:
      value_out = frame.r9;
      return true;
    default:
      break;
  }
  std::uint32_t offset = 0;
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1:
      offset = 0x28;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1:
      offset = 0x30;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1:
      offset = 0x38;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1:
      offset = 0x40;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1:
      offset = 0x48;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1:
      offset = 0x50;
      break;
    default:
      return false;
  }
  if (frame.original_rsp == 0 ||
      frame.original_rsp > std::numeric_limits<std::uintptr_t>::max() - offset ||
      !readable_range(frame.original_rsp + offset, sizeof(value_out))) {
    return false;
  }
  std::memcpy(
      &value_out,
      reinterpret_cast<const void*>(frame.original_rsp + offset),
      sizeof(value_out));
  return true;
}

bool extract_registration_entry(
    const std::size_t hook_index,
    const RegistrationEntryFrame& frame,
    ExtractedRegistrationEntry& result) noexcept {
  if (hook_index >= registration::kPinnedRegistrationHooks.size() || frame.rcx == 0 ||
      !readable_range(frame.rcx, sizeof(std::uintptr_t))) {
    return false;
  }
  const auto& hook = registration::kPinnedRegistrationHooks[hook_index];
  ExtractedRegistrationEntry extracted{};
  extracted.kind = hook.kind;
  extracted.engine_capability = frame.rcx;
  extracted.argument_count = hook.argument_count;
  for (std::size_t index = 0; index < hook.argument_count; ++index) {
    const auto& contract = hook.arguments[index];
    auto& projection = extracted.arguments[index];
    projection.semantic = contract.semantic;
    std::uintptr_t value = 0;
    if (!registration_argument_value(frame, contract.source, value)) return false;
    switch (contract.semantic) {
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1:
        if (!copy_bounded_utf8(value, projection)) return false;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1: {
        constexpr auto bytes =
            adapter::layout_v23300::donor::function_pointer_descriptor_bytes;
        constexpr auto flag_offset =
            adapter::layout_v23300::donor::function_pointer_descriptor_flag;
        if (!readable_range(value, bytes)) return false;
        std::memcpy(
            projection.opaque_descriptor.data(),
            reinterpret_cast<const void*>(value),
            bytes);
        const auto flag = std::to_integer<std::uint8_t>(
            projection.opaque_descriptor[flag_offset]);
        if (flag == 0 || flag > 3) return false;
        projection.opaque_descriptor_bytes = static_cast<std::uint32_t>(bytes);
        projection.pointer_capability = value;
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1: {
        constexpr auto bytes =
            adapter::layout_v23300::donor::function_caller_descriptor_bytes;
        constexpr auto type_offset =
            adapter::layout_v23300::donor::function_caller_descriptor_type;
        if (!readable_range(value, bytes)) return false;
        std::memcpy(
            projection.opaque_descriptor.data(),
            reinterpret_cast<const void*>(value),
            bytes);
        std::int32_t type = 0;
        std::memcpy(
            &type,
            projection.opaque_descriptor.data() + type_offset,
            sizeof(type));
        std::uintptr_t callable = 0;
        std::memcpy(&callable, projection.opaque_descriptor.data(), sizeof(callable));
        if (type < 0 || type > 2 || (type == 0) != (callable == 0)) return false;
        projection.scalar = static_cast<std::uint32_t>(type);
        projection.pointer_capability = callable;
        projection.opaque_descriptor_bytes = static_cast<std::uint32_t>(bytes);
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1:
        if (static_cast<std::uint32_t>(value) > 8) return false;
        projection.scalar = static_cast<std::uint32_t>(value);
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1:
        if (static_cast<std::uint8_t>(value) > 1) return false;
        projection.scalar = static_cast<std::uint8_t>(value);
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1:
        projection.pointer_capability = value;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1:
        projection.scalar = static_cast<std::uint32_t>(value);
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1:
        projection.scalar = static_cast<std::uint32_t>(value);
        break;
      default:
        return false;
    }
  }
  result = extracted;
  return true;
}

class RegistrationOrderTracker final {
 public:
  static constexpr std::size_t kMaximumDepth = 32;

  explicit RegistrationOrderTracker(const DWORD owner_thread) noexcept
      : owner_thread_(owner_thread) {}

  bool enter(
      const std::uint32_t kind,
      const std::uint64_t token,
      std::uint64_t& sequence_out) noexcept {
    if (GetCurrentThreadId() != owner_thread_ || kind == 0 || token == 0 ||
        depth_ == pending_.size() || next_sequence_ == std::numeric_limits<std::uint64_t>::max()) {
      return false;
    }
    sequence_out = next_sequence_++;
    pending_[depth_++] = {kind, token, sequence_out};
    return true;
  }

  bool leave(
      const std::uint32_t kind,
      const std::int32_t eax_result,
      std::uint64_t& token_out,
      std::uint64_t& sequence_out) noexcept {
    if (GetCurrentThreadId() != owner_thread_ || depth_ == 0 ||
        pending_[depth_ - 1].kind != kind) {
      return false;
    }
    const auto completed = pending_[--depth_];
    token_out = completed.token;
    sequence_out = completed.sequence;
    last_result_ = eax_result;
    return true;
  }

  [[nodiscard]] std::size_t depth() const noexcept { return depth_; }
  [[nodiscard]] std::int32_t last_result() const noexcept { return last_result_; }

 private:
  struct Pending final {
    std::uint32_t kind{};
    std::uint64_t token{};
    std::uint64_t sequence{};
  };
  DWORD owner_thread_{};
  std::uint64_t next_sequence_{};
  std::array<Pending, kMaximumDepth> pending_{};
  std::size_t depth_{};
  std::int32_t last_result_{};
};

bool extract_build_jit_metadata(
    const std::uintptr_t manager,
    const std::uint32_t build_identifier_from_eax,
    const std::uintptr_t static_jit_info_from_rax,
    gore_as_capture_build_jit_v1& result) noexcept {
  if (manager == 0 || build_identifier_from_eax != target::kBuildIdentifier ||
      !readable_range(
          manager + adapter::kManagerPrecompiledDataOffset, sizeof(std::uintptr_t))) {
    return false;
  }
  std::uintptr_t precompiled = 0;
  std::memcpy(
      &precompiled,
      reinterpret_cast<const void*>(manager + adapter::kManagerPrecompiledDataOffset),
      sizeof(precompiled));
  if (precompiled == 0 ||
      !readable_range(
          precompiled + adapter::kPrecompiledDataGuidOffset, target::kPrecompiledGuid.size())) {
    return false;
  }

  result = {};
  result.struct_size = sizeof(result);
  result.build_identifier = build_identifier_from_eax;
  std::memcpy(
      result.precompiled_guid,
      reinterpret_cast<const void*>(precompiled + adapter::kPrecompiledDataGuidOffset),
      sizeof(result.precompiled_guid));
  result.shipping_cache_matches =
      std::memcmp(
          result.precompiled_guid,
          target::kPrecompiledGuid.data(),
          target::kPrecompiledGuid.size()) == 0
          ? 1u
          : 0u;
  if (static_jit_info_from_rax != 0) {
    if (!readable_range(static_jit_info_from_rax, sizeof(result.compiled_jit_guid))) {
      return false;
    }
    result.jit_info_present = 1;
    std::memcpy(
        result.compiled_jit_guid,
        reinterpret_cast<const void*>(static_jit_info_from_rax),
        sizeof(result.compiled_jit_guid));
    result.jit_guid_matches =
        std::memcmp(
            result.precompiled_guid,
            result.compiled_jit_guid,
            sizeof(result.precompiled_guid)) == 0
            ? 1u
            : 0u;
    // The pinned Initialize_AnyThread CFG reaches FJITDatabase::Clear iff the non-null compiled
    // GUID differs from PrecompiledData::DataGuid (RVA 0x46859c3..0x46859f9).
    result.jit_database_cleared = result.jit_guid_matches == 0 ? 1u : 0u;
  }
  result.as_reference_debugging = adapter::kAsReferenceDebugging ? 1u : 0u;
  result.fork_opcode_table_201_212_present =
      adapter::kForkOpcodeTable201Through212Present ? 1u : 0u;
  result.reference_debug_opcodes_emittable =
      adapter::kReferenceDebugOpcodesEmittable ? 1u : 0u;
  result.resolve_object_ptr_callback_registered =
      adapter::kResolveObjectPtrCallbackRegistered ? 1u : 0u;
  result.get_build_identifier_rva = target::kRvaGetBuildIdentifier;
  result.get_static_jit_info_rva = target::kRvaGetStaticJitInfo;
  return true;
}

bool extract_initial_compile_entry(
    const std::uintptr_t manager_from_rcx,
    TypedFrontendMetadata& result) noexcept {
  if (manager_from_rcx == 0 ||
      !readable_range(
          manager_from_rcx + adapter::kManagerEngineOffset, sizeof(std::uintptr_t))) {
    return false;
  }
  std::uintptr_t engine = 0;
  std::memcpy(
      &engine,
      reinterpret_cast<const void*>(manager_from_rcx + adapter::kManagerEngineOffset),
      sizeof(engine));
  if (engine == 0 || !readable_range(engine, sizeof(std::uintptr_t))) return false;
  result = {};
  result.manager_capability = manager_from_rcx;
  result.engine_capability = engine;
  return true;
}

bool extract_precompiled_descriptor_result(
    const std::uintptr_t precompiled_from_rcx,
    const std::uintptr_t output_array_from_rdx,
    const std::uintptr_t returned_array_from_rax,
    TypedFrontendMetadata& result) noexcept {
  if (precompiled_from_rcx == 0 || output_array_from_rdx == 0 ||
      returned_array_from_rax != output_array_from_rdx ||
      !readable_range(precompiled_from_rcx, sizeof(std::uintptr_t)) ||
      !readable_range(output_array_from_rdx, 16)) {
    return false;
  }
  std::uintptr_t data = 0;
  std::int32_t count = 0;
  std::int32_t capacity = 0;
  std::memcpy(&data, reinterpret_cast<const void*>(output_array_from_rdx), sizeof(data));
  std::memcpy(
      &count,
      reinterpret_cast<const void*>(output_array_from_rdx + sizeof(data)),
      sizeof(count));
  std::memcpy(
      &capacity,
      reinterpret_cast<const void*>(output_array_from_rdx + sizeof(data) + sizeof(count)),
      sizeof(capacity));
  if (count < 0 || capacity < count || (count != 0 && data == 0)) return false;
  const auto unsigned_count = static_cast<std::uint32_t>(count);
  if (unsigned_count > target::kMaxRecords ||
      unsigned_count > std::numeric_limits<std::size_t>::max() /
                           adapter::kPrecompiledDescriptorStride ||
      (unsigned_count != 0 &&
       !readable_range(
           data,
           static_cast<std::size_t>(unsigned_count) *
               adapter::kPrecompiledDescriptorStride))) {
    return false;
  }
  result = {};
  result.boundary_object_capability = output_array_from_rdx;
  result.item_count = unsigned_count;
  return true;
}

bool extract_preprocessor_constructed(
    const std::uintptr_t preprocessor_from_saved_rcx,
    TypedFrontendMetadata& result) noexcept {
  // The pinned constructor writes through offset 0x100. This proves object construction only;
  // no field is interpreted as a config value until its serializer is separately mapped.
  constexpr std::size_t kMinimumConstructedBytes = 0x108;
  if (!readable_range(preprocessor_from_saved_rcx, kMinimumConstructedBytes)) return false;
  result = {};
  result.boundary_object_capability = preprocessor_from_saved_rcx;
  return true;
}

bool extract_initial_compile_return(
    const std::uintptr_t manager_from_rbx,
    TypedFrontendMetadata& result) noexcept {
  if (manager_from_rbx == 0 ||
      !readable_range(
          manager_from_rbx + adapter::kManagerInitialCompileSucceededOffset, 1)) {
    return false;
  }
  std::uint8_t succeeded = 0;
  std::memcpy(
      &succeeded,
      reinterpret_cast<const void*>(
          manager_from_rbx + adapter::kManagerInitialCompileSucceededOffset),
      sizeof(succeeded));
  if (succeeded > 1) return false;
  result = {};
  result.manager_capability = manager_from_rbx;
  result.initial_compile_succeeded = succeeded != 0;
  return true;
}

bool extract_bind_metadata(
    const std::uintptr_t image,
    const std::uintptr_t manager,
    const std::uintptr_t record,
    const std::uintptr_t end,
    const std::uintptr_t callback,
    TypedBindMetadata& result) noexcept {
  if (image == 0 || manager == 0 || record == 0 || end == 0 || callback <= image ||
      callback - image >= target::kPeSizeOfImage ||
      record > std::numeric_limits<std::uintptr_t>::max() - adapter::kBindRecordStride ||
      record + adapter::kBindRecordStride > end ||
      !readable_range(manager + adapter::kManagerEngineOffset, sizeof(std::uintptr_t)) ||
      !readable_range(record + adapter::kBindOrderOffset, sizeof(std::int32_t))) {
    return false;
  }
  std::uintptr_t engine = 0;
  std::int32_t order = 0;
  std::memcpy(
      &engine,
      reinterpret_cast<const void*>(manager + adapter::kManagerEngineOffset),
      sizeof(engine));
  std::memcpy(
      &order,
      reinterpret_cast<const void*>(record + adapter::kBindOrderOffset),
      sizeof(order));
  if (engine == 0 || !readable_range(engine, sizeof(std::uintptr_t))) return false;
  result.bind_order = order;
  result.callback_rva = static_cast<std::uint32_t>(callback - image);
  result.final_callback = record + adapter::kBindRecordStride == end;
  result.engine_capability = engine;
  return true;
}

class ThreadWindow final {
 public:
  ThreadWindow() = default;
  ~ThreadWindow() { release(); }
  ThreadWindow(const ThreadWindow&) = delete;
  ThreadWindow& operator=(const ThreadWindow&) = delete;

  bool acquire() noexcept {
    if (active_) return false;
    try {
      if (!enumerate_and_suspend() || !no_new_threads()) {
        release();
        return false;
      }
      active_ = true;
      return true;
    } catch (...) {
      release();
      return false;
    }
  }

  bool all_instruction_pointers_outside(
      const std::span<const std::pair<std::uintptr_t, std::uintptr_t>> ranges) const noexcept {
    if (!active_) return false;
    for (const auto& entry : threads_) {
      CONTEXT context{};
      context.ContextFlags = CONTEXT_CONTROL;
      if (GetThreadContext(entry.handle, &context) == FALSE) return false;
      const auto rip = static_cast<std::uintptr_t>(context.Rip);
      for (const auto& [begin, end] : ranges) {
        if (begin >= end || (rip >= begin && rip < end)) return false;
      }
    }
    return true;
  }

  void release() noexcept {
    for (auto entry = threads_.rbegin(); entry != threads_.rend(); ++entry) {
      if (entry->suspended) (void)ResumeThread(entry->handle);
      if (entry->handle != nullptr) (void)CloseHandle(entry->handle);
    }
    threads_.clear();
    active_ = false;
  }

 private:
  struct Entry final {
    HANDLE handle{};
    DWORD thread_id{};
    bool suspended{};
  };

  bool enumerate_and_suspend() {
    const DWORD process_id = GetCurrentProcessId();
    const DWORD current_thread = GetCurrentThreadId();
    const HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    THREADENTRY32 item{};
    item.dwSize = sizeof(item);
    bool ok = Thread32First(snapshot, &item) != FALSE;
    while (ok) {
      if (item.th32OwnerProcessID == process_id && item.th32ThreadID != current_thread) {
        const HANDLE thread = OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_LIMITED_INFORMATION,
            FALSE,
            item.th32ThreadID);
        if (thread == nullptr) {
          (void)CloseHandle(snapshot);
          return false;
        }
        try {
          threads_.push_back({thread, item.th32ThreadID, false});
        } catch (...) {
          (void)CloseHandle(thread);
          (void)CloseHandle(snapshot);
          throw;
        }
        if (SuspendThread(thread) == std::numeric_limits<DWORD>::max()) {
          (void)CloseHandle(snapshot);
          return false;
        }
        threads_.back().suspended = true;
      }
      item.dwSize = sizeof(item);
      ok = Thread32Next(snapshot, &item) != FALSE;
    }
    const DWORD enumeration_error = GetLastError();
    (void)CloseHandle(snapshot);
    return enumeration_error == ERROR_NO_MORE_FILES;
  }

  bool no_new_threads() const noexcept {
    const DWORD process_id = GetCurrentProcessId();
    const DWORD current_thread = GetCurrentThreadId();
    const HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    THREADENTRY32 item{};
    item.dwSize = sizeof(item);
    bool ok = Thread32First(snapshot, &item) != FALSE;
    bool stable = true;
    while (ok) {
      if (item.th32OwnerProcessID == process_id && item.th32ThreadID != current_thread) {
        const auto found = std::find_if(
            threads_.begin(), threads_.end(), [&](const Entry& entry) {
              return entry.thread_id == item.th32ThreadID;
            });
        if (found == threads_.end()) {
          stable = false;
          break;
        }
      }
      item.dwSize = sizeof(item);
      ok = Thread32Next(snapshot, &item) != FALSE;
    }
    const DWORD enumeration_error = GetLastError();
    (void)CloseHandle(snapshot);
    return stable && (!ok && enumeration_error == ERROR_NO_MORE_FILES);
  }

  std::vector<Entry> threads_;
  bool active_{};
};

template <typename Type>
const Type* checked_image_object(
    const std::uintptr_t image,
    const std::uint32_t image_size,
    const std::uint32_t rva,
    const std::size_t count = 1) noexcept {
  if (count == 0 || count > std::numeric_limits<std::size_t>::max() / sizeof(Type)) {
    return nullptr;
  }
  const std::size_t bytes = count * sizeof(Type);
  if (rva >= image_size || bytes > image_size - rva || image >
          std::numeric_limits<std::uintptr_t>::max() - rva) {
    return nullptr;
  }
  const std::uintptr_t address = image + rva;
  return readable_range(address, bytes) ? reinterpret_cast<const Type*>(address) : nullptr;
}

bool pinned_frontend_accessor(
    const std::uintptr_t image,
    const std::uint32_t accessor_rva,
    const std::uint32_t delegate_rva) noexcept {
  constexpr std::size_t kInstructionBytes = 7;
  const auto* instruction = checked_image_object<std::byte>(
      image, target::kPeSizeOfImage, accessor_rva, kInstructionBytes);
  if (instruction == nullptr || instruction[0] != std::byte{0x48} ||
      instruction[1] != std::byte{0x8d} || instruction[2] != std::byte{0x05}) {
    return false;
  }
  std::int32_t displacement = 0;
  std::memcpy(&displacement, instruction + 3, sizeof(displacement));
  return static_cast<std::int64_t>(accessor_rva) +
             static_cast<std::int64_t>(kInstructionBytes) +
             static_cast<std::int64_t>(displacement) ==
         static_cast<std::int64_t>(delegate_rva);
}

std::uint32_t validate_current_image(const std::uintptr_t image) noexcept {
  if (image == 0 || reinterpret_cast<HMODULE>(image) != GetModuleHandleW(nullptr) ||
      !readable_range(image, sizeof(IMAGE_DOS_HEADER))) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(image);
  if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 ||
      static_cast<std::uint32_t>(dos->e_lfanew) > target::kPeSizeOfImage -
                                                         sizeof(IMAGE_NT_HEADERS64)) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  const auto* nt = checked_image_object<IMAGE_NT_HEADERS64>(
      image,
      target::kPeSizeOfImage,
      static_cast<std::uint32_t>(dos->e_lfanew));
  if (nt == nullptr || nt->Signature != IMAGE_NT_SIGNATURE ||
      nt->FileHeader.Machine != IMAGE_FILE_MACHINE_AMD64 ||
      nt->FileHeader.SizeOfOptionalHeader != sizeof(IMAGE_OPTIONAL_HEADER64) ||
      nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC ||
      nt->OptionalHeader.NumberOfRvaAndSizes <= IMAGE_DIRECTORY_ENTRY_DEBUG ||
      nt->OptionalHeader.DllCharacteristics != adapter::kTargetDllCharacteristics ||
      nt->OptionalHeader.SizeOfImage != target::kPeSizeOfImage) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  const auto& load_config_directory =
      nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_LOAD_CONFIG];
  constexpr std::size_t required_load_config_bytes =
      offsetof(IMAGE_LOAD_CONFIG_DIRECTORY64, GuardFlags) + sizeof(std::uint32_t);
  const auto* load_config = checked_image_object<IMAGE_LOAD_CONFIG_DIRECTORY64>(
      image, target::kPeSizeOfImage, load_config_directory.VirtualAddress);
  if (load_config_directory.VirtualAddress == 0 ||
      load_config_directory.Size < required_load_config_bytes || load_config == nullptr ||
      load_config->Size < required_load_config_bytes ||
      load_config->GuardFlags != adapter::kTargetGuardFlags ||
      load_config->GuardCFFunctionTable != 0 || load_config->GuardCFFunctionCount != 0) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  const auto& debug_directory =
      nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG];
  if (debug_directory.VirtualAddress == 0 || debug_directory.Size == 0 ||
      debug_directory.Size % sizeof(IMAGE_DEBUG_DIRECTORY) != 0) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  const auto debug_count = debug_directory.Size / sizeof(IMAGE_DEBUG_DIRECTORY);
  const auto* entries = checked_image_object<IMAGE_DEBUG_DIRECTORY>(
      image, target::kPeSizeOfImage, debug_directory.VirtualAddress, debug_count);
  bool codeview_matches = false;
  if (entries != nullptr) {
    for (std::size_t index = 0; index < debug_count; ++index) {
      const auto& entry = entries[index];
      if (entry.Type != IMAGE_DEBUG_TYPE_CODEVIEW || entry.AddressOfRawData == 0 ||
          entry.SizeOfData < 24) {
        continue;
      }
      const auto* codeview = checked_image_object<std::byte>(
          image, target::kPeSizeOfImage, entry.AddressOfRawData, entry.SizeOfData);
      if (codeview == nullptr || std::memcmp(codeview, "RSDS", 4) != 0) {
        continue;
      }
      std::uint32_t age = 0;
      std::memcpy(&age, codeview + 20, sizeof(age));
      if (age == target::kCodeViewAge &&
          std::memcmp(codeview + 4, target::kCodeViewGuidRsds.data(), 16) == 0) {
        codeview_matches = true;
        break;
      }
    }
  }
  if (!codeview_matches) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_TARGET_V1;
  }
  for (const auto& site : adapter::kPinnedInstructionSpans) {
    const auto* actual = checked_image_object<std::byte>(
        image, target::kPeSizeOfImage, site.patch_anchor_rva, site.byte_count);
    if (actual == nullptr ||
        std::memcmp(actual, site.expected.data(), site.byte_count) != 0) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    }
  }
  for (std::size_t index = 0; index < registration::kPinnedRegistrationHooks.size(); ++index) {
    const auto& site = registration::kPinnedRegistrationHooks[index];
    const auto* actual = checked_image_object<std::byte>(
        image, target::kPeSizeOfImage, site.function_rva, site.overwrite_bytes);
    const auto* vtable_entry = checked_image_object<std::uintptr_t>(
        image,
        target::kPeSizeOfImage,
        registration::kEngineVtableRva +
            site.vtable_slot * static_cast<std::uint32_t>(sizeof(std::uintptr_t)));
    if (actual == nullptr || vtable_entry == nullptr ||
        std::memcmp(actual, site.expected.data(), site.overwrite_bytes) != 0 ||
        image > std::numeric_limits<std::uintptr_t>::max() - site.function_rva ||
        *vtable_entry != image + site.function_rva) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    }
    DWORD64 runtime_image_base = 0;
    const auto* runtime_function = RtlLookupFunctionEntry(
        static_cast<DWORD64>(image + site.function_rva), &runtime_image_base, nullptr);
    if (runtime_function == nullptr || runtime_image_base != image ||
        runtime_function->BeginAddress != registration::kRegistrationTarget.function_rvas[index] ||
        runtime_function->EndAddress != registration::kRegistrationTarget.function_end_rvas[index] ||
        runtime_function->UnwindData !=
            registration::kRegistrationTarget.source_unwind_info_rvas[index]) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    }
  }
  for (const auto& site : adapter::frontend_target_layout::callback_callsites) {
    const auto* actual = checked_image_object<std::byte>(
        image,
        target::kPeSizeOfImage,
        site.call_rva,
        site.expected_call.size());
    if (actual == nullptr ||
        std::memcmp(actual, site.expected_call.data(), site.expected_call.size()) != 0 ||
        site.return_rva != site.call_rva + site.expected_call.size() ||
        static_cast<std::int64_t>(site.return_rva) + site.relative_displacement !=
            site.direct_callee_rva) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    }
  }
  for (const auto accessor_rva :
       adapter::frontend_target_layout::kFrontendTarget.class_analyze_accessor_rvas) {
    if (!pinned_frontend_accessor(
            image,
            accessor_rva,
            adapter::frontend_target_layout::class_analyze_delegate_rva)) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
    }
  }
  return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
}

enum class FixtureEvent : std::uint8_t {
  engine_property,
  bind_call,
  bind_return,
  registry_support,
  final_state,
  build_identifier,
  static_jit,
  frontend_configs,
  initial_compile_enter,
  frontend_middle,
  initial_compile_return,
  seal,
};

class RecordOrder final {
 public:
  bool append(const FixtureEvent event) noexcept {
    switch (event) {
      case FixtureEvent::engine_property:
        return phase_ == Phase::properties;
      case FixtureEvent::bind_call:
        if (phase_ != Phase::properties && phase_ != Phase::between_callbacks) return false;
        phase_ = Phase::in_callback;
        return true;
      case FixtureEvent::bind_return:
        if (phase_ != Phase::in_callback) return false;
        phase_ = Phase::between_callbacks;
        return true;
      case FixtureEvent::registry_support:
        if (phase_ != Phase::between_callbacks) return false;
        phase_ = Phase::support;
        return true;
      case FixtureEvent::final_state:
        if (phase_ != Phase::support) return false;
        phase_ = Phase::final_state;
        return true;
      case FixtureEvent::build_identifier:
        if (phase_ != Phase::final_state) return false;
        phase_ = Phase::build_identifier;
        return true;
      case FixtureEvent::static_jit:
        if (phase_ != Phase::build_identifier) return false;
        phase_ = Phase::static_jit;
        return true;
      case FixtureEvent::frontend_configs:
        if (phase_ != Phase::static_jit) return false;
        phase_ = Phase::frontend_configs;
        return true;
      case FixtureEvent::initial_compile_enter:
        if (phase_ != Phase::frontend_configs) return false;
        phase_ = Phase::frontend_enter;
        return true;
      case FixtureEvent::frontend_middle:
        if (phase_ != Phase::frontend_enter) return false;
        phase_ = Phase::frontend_middle;
        return true;
      case FixtureEvent::initial_compile_return:
        if (phase_ != Phase::frontend_middle) return false;
        phase_ = Phase::frontend_return;
        return true;
      case FixtureEvent::seal:
        if (phase_ != Phase::frontend_return) return false;
        phase_ = Phase::sealed;
        return true;
    }
    return false;
  }

 private:
  enum class Phase : std::uint8_t {
    properties,
    in_callback,
    between_callbacks,
    support,
    final_state,
    build_identifier,
    static_jit,
    frontend_configs,
    frontend_enter,
    frontend_middle,
    frontend_return,
    sealed,
  };
  Phase phase_{Phase::properties};
};

#if defined(GORE_AS_CAPTURE_TEST_TARGET)

class FixturePage final {
 public:
  FixturePage() noexcept
      : bytes_(static_cast<std::byte*>(VirtualAlloc(
            nullptr, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE))) {}
  ~FixturePage() {
    if (bytes_ != nullptr) {
      (void)VirtualFree(bytes_, 0, MEM_RELEASE);
    }
  }
  FixturePage(const FixturePage&) = delete;
  FixturePage& operator=(const FixturePage&) = delete;
  [[nodiscard]] std::byte* get() const noexcept { return bytes_; }

 private:
  std::byte* bytes_{};
};

class RegistrationFixturePage final {
 public:
  static constexpr std::size_t kBytes = 64 * 1024;

  RegistrationFixturePage() noexcept
      : bytes_(static_cast<std::byte*>(VirtualAlloc(
            nullptr, kBytes, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE))) {}
  ~RegistrationFixturePage() {
    if (bytes_ != nullptr) (void)VirtualFree(bytes_, 0, MEM_RELEASE);
  }
  RegistrationFixturePage(const RegistrationFixturePage&) = delete;
  RegistrationFixturePage& operator=(const RegistrationFixturePage&) = delete;
  [[nodiscard]] std::byte* get() const noexcept { return bytes_; }

 private:
  std::byte* bytes_{};
};

struct RegistrationTrampolinePlan final {
  std::array<std::byte, 64> bytes{};
  std::uint8_t byte_count{};
};

bool build_registration_trampoline_plan(
    const std::size_t index,
    const std::uintptr_t original,
    RegistrationTrampolinePlan& plan) noexcept {
  if (index >= registration::kPinnedRegistrationHooks.size()) return false;
  const auto& site = registration::kPinnedRegistrationHooks[index];
  if (original > std::numeric_limits<std::uintptr_t>::max() - site.overwrite_bytes) {
    return false;
  }
  plan = {};
  std::memcpy(plan.bytes.data(), site.expected.data(), site.overwrite_bytes);
  plan.byte_count = site.overwrite_bytes;
  TrampolinePlan tail{};
  tail.byte_count = plan.byte_count;
  std::memcpy(tail.bytes.data(), plan.bytes.data(), plan.byte_count);
  if (!append_indirect_jump(tail, original + site.overwrite_bytes)) return false;
  plan.byte_count = tail.byte_count;
  std::memcpy(plan.bytes.data(), tail.bytes.data(), tail.byte_count);
  return true;
}

class FixtureRegistrationPatchTransaction final {
 public:
  static constexpr std::size_t kSiteStride = 64;
  static constexpr std::size_t kRelayBase = 0x1000;
  static constexpr std::size_t kRelayStride = 16;
  static constexpr std::size_t kTrampolineBase = 0x2000;
  static constexpr std::size_t kTrampolineStride = 64;
  static constexpr std::size_t kNoFailure = std::numeric_limits<std::size_t>::max();

  explicit FixtureRegistrationPatchTransaction(std::byte* const page) noexcept
      : page_(page) {
    if (page_ == nullptr) return;
    for (std::size_t index = 0; index < registration::kPinnedRegistrationHooks.size();
         ++index) {
      const auto& pinned = registration::kPinnedRegistrationHooks[index];
      expected_[index] = pinned.expected;
      replacement_[index].fill(std::byte{0x90});
      constexpr std::array<std::byte, 6> indirect_jump{
          std::byte{0xff}, std::byte{0x25}, std::byte{0x00},
          std::byte{0x00}, std::byte{0x00}, std::byte{0x00}};
      std::memcpy(replacement_[index].data(), indirect_jump.data(), indirect_jump.size());
      const auto wrapper = reinterpret_cast<std::uintptr_t>(relay(index));
      std::memcpy(replacement_[index].data() + indirect_jump.size(), &wrapper, sizeof(wrapper));
      std::memcpy(address(index), expected_[index].data(), pinned.overwrite_bytes);

      RegistrationTrampolinePlan trampoline{};
      if (!build_registration_trampoline_plan(
              index, reinterpret_cast<std::uintptr_t>(address(index)), trampoline)) {
        return;
      }
      trampolines_[index] = trampoline;
      std::memcpy(
          trampoline_address(index), trampoline.bytes.data(), trampoline.byte_count);
      relay(index)[0] = std::byte{0xc3};
    }
    initialized_ = true;
  }

  std::uint32_t install(const std::size_t fail_at = kNoFailure) noexcept {
    if (!initialized_) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    if (installed_) return GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1;
    for (std::size_t index = 0; index < expected_.size(); ++index) {
      const auto bytes = registration::kPinnedRegistrationHooks[index].overwrite_bytes;
      if (std::memcmp(address(index), expected_[index].data(), bytes) != 0) {
        return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
      }
    }
    std::size_t patched = 0;
    for (; patched < expected_.size() && patched != fail_at; ++patched) {
      const auto bytes = registration::kPinnedRegistrationHooks[patched].overwrite_bytes;
      std::memcpy(address(patched), replacement_[patched].data(), bytes);
    }
    if (patched != expected_.size()) {
      while (patched > 0) {
        --patched;
        const auto bytes = registration::kPinnedRegistrationHooks[patched].overwrite_bytes;
        std::memcpy(address(patched), expected_[patched].data(), bytes);
      }
      (void)FlushInstructionCache(GetCurrentProcess(), page_, RegistrationFixturePage::kBytes);
      return GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
    }
    installed_ = true;
    owner_thread_ = GetCurrentThreadId();
    (void)FlushInstructionCache(GetCurrentProcess(), page_, RegistrationFixturePage::kBytes);
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  }

  std::uint32_t uninstall() noexcept {
    if (!installed_) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    if (owner_thread_ != GetCurrentThreadId()) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1;
    }
    for (std::size_t index = 0; index < expected_.size(); ++index) {
      const auto bytes = registration::kPinnedRegistrationHooks[index].overwrite_bytes;
      if (std::memcmp(address(index), replacement_[index].data(), bytes) != 0) {
        return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
      }
    }
    for (std::size_t index = expected_.size(); index > 0; --index) {
      const auto current = index - 1;
      const auto bytes = registration::kPinnedRegistrationHooks[current].overwrite_bytes;
      std::memcpy(address(current), expected_[current].data(), bytes);
    }
    installed_ = false;
    owner_thread_ = 0;
    (void)FlushInstructionCache(GetCurrentProcess(), page_, RegistrationFixturePage::kBytes);
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  }

  [[nodiscard]] bool all_expected() const noexcept {
    for (std::size_t index = 0; index < expected_.size(); ++index) {
      if (std::memcmp(
              address(index),
              expected_[index].data(),
              registration::kPinnedRegistrationHooks[index].overwrite_bytes) != 0) {
        return false;
      }
    }
    return true;
  }

  [[nodiscard]] bool all_replaced() const noexcept {
    for (std::size_t index = 0; index < replacement_.size(); ++index) {
      if (std::memcmp(
              address(index),
              replacement_[index].data(),
              registration::kPinnedRegistrationHooks[index].overwrite_bytes) != 0) {
        return false;
      }
    }
    return true;
  }

  [[nodiscard]] std::byte* address(const std::size_t index) const noexcept {
    return page_ + index * kSiteStride;
  }

  [[nodiscard]] std::byte* relay(const std::size_t index) const noexcept {
    return page_ + kRelayBase + index * kRelayStride;
  }

  [[nodiscard]] std::byte* trampoline_address(const std::size_t index) const noexcept {
    return page_ + kTrampolineBase + index * kTrampolineStride;
  }

  [[nodiscard]] const RegistrationTrampolinePlan& trampoline(
      const std::size_t index) const noexcept {
    return trampolines_[index];
  }

 private:
  std::byte* page_{};
  std::array<std::array<std::byte, 24>, 14> expected_{};
  std::array<std::array<std::byte, 24>, 14> replacement_{};
  std::array<RegistrationTrampolinePlan, 14> trampolines_{};
  DWORD owner_thread_{};
  bool initialized_{};
  bool installed_{};
};

class FixturePatchTransaction final {
 public:
  static constexpr std::size_t kMaxSiteBytes = 16;
  static constexpr std::size_t kSiteStride = 32;
  static constexpr std::size_t kRelayBase = 0x400;
  static constexpr std::size_t kRelayStride = 16;
  static constexpr std::size_t kNoFailure = std::numeric_limits<std::size_t>::max();

  explicit FixturePatchTransaction(std::byte* page) noexcept : page_(page) {
    for (std::size_t site = 0; site < adapter::kPinnedInstructionSpans.size(); ++site) {
      const auto& pinned = adapter::kPinnedInstructionSpans[site];
      expected_[site] = pinned.expected;
      replacement_[site].fill(std::byte{0x90});
      replacement_[site][0] = adapter::kStaticSiteContracts[site].transfer_kind ==
                                      GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1
                                  ? std::byte{0xe8}
                                  : std::byte{0xe9};
      std::int32_t displacement = 0;
      const auto source_end = reinterpret_cast<std::uintptr_t>(address(site)) + 5;
      const auto destination = reinterpret_cast<std::uintptr_t>(relay(site));
      if (!relative_displacement(source_end, destination, displacement)) return;
      std::memcpy(replacement_[site].data() + 1, &displacement, sizeof(displacement));
      std::memcpy(address(site), expected_[site].data(), pinned.byte_count);

      TrampolinePlan trampoline{};
      const auto original = reinterpret_cast<std::uintptr_t>(address(site));
      const auto relocated = reinterpret_cast<std::uintptr_t>(page_ + 0x800 + site * 64);
      if (!build_trampoline_plan(site, original, relocated, trampoline)) return;
      trampolines_[site] = trampoline;
      std::memcpy(
          reinterpret_cast<void*>(relocated), trampoline.bytes.data(), trampoline.byte_count);

      constexpr std::array<std::byte, 2> tail{std::byte{0xff}, std::byte{0xe0}};
      relay(site)[0] = std::byte{0x48};
      relay(site)[1] = std::byte{0xb8};
      std::memcpy(relay(site) + 2, &relocated, sizeof(relocated));
      std::memcpy(relay(site) + 10, tail.data(), tail.size());
    }
    initialized_ = true;
  }

  std::uint32_t install(const std::size_t fail_at = kNoFailure) noexcept {
    if (!initialized_) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    if (installed_) return GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1;
    for (std::size_t site = 0; site < expected_.size(); ++site) {
      const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
      if (std::memcmp(address(site), expected_[site].data(), bytes) != 0) {
        return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
      }
    }
    DWORD previous = 0;
    if (VirtualProtect(page_, 4096, PAGE_EXECUTE_READWRITE, &previous) == FALSE) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
    }
    std::size_t patched = 0;
    for (; patched < expected_.size(); ++patched) {
      if (patched == fail_at) break;
      const auto bytes = adapter::kPinnedInstructionSpans[patched].byte_count;
      std::memcpy(address(patched), replacement_[patched].data(), bytes);
    }
    if (patched != expected_.size()) {
      while (patched > 0) {
        --patched;
        const auto bytes = adapter::kPinnedInstructionSpans[patched].byte_count;
        std::memcpy(address(patched), expected_[patched].data(), bytes);
      }
      (void)FlushInstructionCache(GetCurrentProcess(), page_, 4096);
      DWORD ignored = 0;
      const bool protection_restored =
          VirtualProtect(page_, 4096, previous, &ignored) != FALSE;
      return protection_restored ? GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1
                                 : GORE_AS_CAPTURE_INSTRUMENTATION_ROLLBACK_FAILED_V1;
    }
    (void)FlushInstructionCache(GetCurrentProcess(), page_, 4096);
    DWORD ignored = 0;
    if (VirtualProtect(page_, 4096, previous, &ignored) == FALSE) {
      for (std::size_t site = 0; site < expected_.size(); ++site) {
        const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
        std::memcpy(address(site), expected_[site].data(), bytes);
      }
      (void)FlushInstructionCache(GetCurrentProcess(), page_, 4096);
      return GORE_AS_CAPTURE_INSTRUMENTATION_ROLLBACK_FAILED_V1;
    }
    owner_thread_ = GetCurrentThreadId();
    installed_ = true;
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  }

  std::uint32_t uninstall() noexcept {
    if (!installed_) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    if (owner_thread_ != GetCurrentThreadId()) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1;
    }
    for (std::size_t site = 0; site < replacement_.size(); ++site) {
      const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
      if (std::memcmp(address(site), replacement_[site].data(), bytes) != 0) {
        return GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1;
      }
    }
    DWORD previous = 0;
    if (VirtualProtect(page_, 4096, PAGE_EXECUTE_READWRITE, &previous) == FALSE) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
    }
    for (std::size_t site = 0; site < expected_.size(); ++site) {
      const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
      std::memcpy(address(site), expected_[site].data(), bytes);
    }
    (void)FlushInstructionCache(GetCurrentProcess(), page_, 4096);
    DWORD ignored = 0;
    if (VirtualProtect(page_, 4096, previous, &ignored) == FALSE) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_ROLLBACK_FAILED_V1;
    }
    installed_ = false;
    owner_thread_ = 0;
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  }

  [[nodiscard]] bool all_expected() const noexcept {
    for (std::size_t site = 0; site < expected_.size(); ++site) {
      const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
      if (std::memcmp(address(site), expected_[site].data(), bytes) != 0) return false;
    }
    return true;
  }

  [[nodiscard]] bool all_replaced() const noexcept {
    for (std::size_t site = 0; site < replacement_.size(); ++site) {
      const auto bytes = adapter::kPinnedInstructionSpans[site].byte_count;
      if (std::memcmp(address(site), replacement_[site].data(), bytes) != 0) return false;
      std::int32_t displacement = 0;
      std::memcpy(&displacement, replacement_[site].data() + 1, sizeof(displacement));
      std::uintptr_t decoded = 0;
      if (!add_signed_displacement(
              reinterpret_cast<std::uintptr_t>(address(site)) + 5,
              displacement,
              decoded) ||
          decoded != reinterpret_cast<std::uintptr_t>(relay(site))) {
        return false;
      }
    }
    return true;
  }

  [[nodiscard]] bool installed() const noexcept { return installed_; }
  [[nodiscard]] std::uint32_t prepare_unload() const noexcept {
    return installed_ ? GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1
                      : GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  }
  std::byte* address(const std::size_t site) const noexcept {
    return page_ + site * kSiteStride;
  }
  std::byte* relay(const std::size_t site) const noexcept {
    return page_ + kRelayBase + site * kRelayStride;
  }
  [[nodiscard]] const TrampolinePlan& trampoline(const std::size_t site) const noexcept {
    return trampolines_[site];
  }

 private:
  std::byte* page_{};
  std::array<std::array<std::byte, kMaxSiteBytes>, 9> expected_{};
  std::array<std::array<std::byte, kMaxSiteBytes>, 9> replacement_{};
  std::array<TrampolinePlan, 9> trampolines_{};
  DWORD owner_thread_{};
  bool installed_{};
  bool initialized_{};
};

bool selftest_site_contracts() noexcept {
  for (std::size_t index = 0; index < adapter::kStaticSiteContracts.size(); ++index) {
    const auto& span = adapter::kPinnedInstructionSpans[index];
    const auto& contract = adapter::kStaticSiteContracts[index];
    if (span.kind != target::kPinnedHookTable[index].kind ||
        span.observation_rva != target::kPinnedHookTable[index].image_rva ||
        span.byte_count < 5 ||
        contract.frame_kind != index + 1 ||
        (contract.transfer_kind != GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1 &&
         contract.transfer_kind != GORE_AS_CAPTURE_TRANSFER_INLINE_JUMP_V1 &&
         contract.transfer_kind != GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1)) {
      return false;
    }
  }
  return adapter::kStaticSiteContracts[1].engine_offset ==
             adapter::kManagerEngineOffset &&
         adapter::kStaticSiteContracts[1].record_stride == adapter::kBindRecordStride &&
         adapter::kStaticSiteContracts[6].direct_callee_rva ==
             adapter::kRvaPrecompiledRequestCallee &&
         adapter::kStaticSiteContracts[7].direct_callee_rva ==
             adapter::kRvaPreprocessorConstructor;
}

bool decode_relative_target(
    const TrampolinePlan& plan,
    const std::uintptr_t relocated,
    const std::size_t displacement_offset,
    const std::size_t instruction_end_offset,
    std::uintptr_t& target_out) noexcept {
  if (plan.byte_count < displacement_offset + sizeof(std::int32_t)) return false;
  std::int32_t displacement = 0;
  std::memcpy(
      &displacement, plan.bytes.data() + displacement_offset, sizeof(displacement));
  return add_signed_displacement(
      relocated + instruction_end_offset, displacement, target_out);
}

bool selftest_relocation_plans(const FixturePatchTransaction& transaction) noexcept {
  for (std::size_t index = 0; index < adapter::kPinnedInstructionSpans.size(); ++index) {
    const auto original = reinterpret_cast<std::uintptr_t>(transaction.address(index));
    const auto relocated = reinterpret_cast<std::uintptr_t>(
        transaction.address(0) + 0x800 + index * 64);
    TrampolinePlan rebuilt{};
    if (!build_trampoline_plan(index, original, relocated, rebuilt) ||
        rebuilt.byte_count != transaction.trampoline(index).byte_count ||
        std::memcmp(
            rebuilt.bytes.data(),
            transaction.trampoline(index).bytes.data(),
            rebuilt.byte_count) != 0) {
      return false;
    }
    std::size_t displacement_offset = 0;
    std::size_t instruction_end_offset = 0;
    switch (index) {
      case 0:
        displacement_offset = 7;
        instruction_end_offset = 11;
        break;
      case 4:
        displacement_offset = 3;
        instruction_end_offset = 7;
        break;
      case 6:
      case 7:
        displacement_offset = 1;
        instruction_end_offset = 5;
        break;
      default:
        continue;
    }
    std::int32_t original_displacement = 0;
    std::memcpy(
        &original_displacement,
        adapter::kPinnedInstructionSpans[index].expected.data() + displacement_offset,
        sizeof(original_displacement));
    std::uintptr_t original_target = 0;
    std::uintptr_t relocated_target = 0;
    if (!add_signed_displacement(
            original + instruction_end_offset,
            original_displacement,
            original_target) ||
        !decode_relative_target(
            rebuilt,
            relocated,
            displacement_offset,
            instruction_end_offset,
            relocated_target) ||
        original_target != relocated_target) {
      return false;
    }
  }
  return true;
}

bool selftest_unwind_plan(const FixturePatchTransaction& transaction) noexcept {
  // Relocated code never changes RSP or establishes a frame. The only displaced calls are these
  // three exact spans; an empty x64 UNWIND_INFO therefore describes each generated leaf range.
  constexpr std::uint32_t expected_call_mask = (1u << 1) | (1u << 6) | (1u << 7);
  std::uint32_t actual_call_mask = 0;
  for (std::size_t index = 0; index < adapter::kPinnedInstructionSpans.size(); ++index) {
    const auto& plan = transaction.trampoline(index);
    if (plan.byte_count == 0 || plan.byte_count > plan.bytes.size()) return false;
    if (plan.contains_call) actual_call_mask |= 1u << index;
    if (index != 4) {
      if (plan.byte_count < 14 || plan.bytes[plan.byte_count - 14] != std::byte{0xff} ||
          plan.bytes[plan.byte_count - 13] != std::byte{0x25}) {
        return false;
      }
    }
  }
  if (actual_call_mask != expected_call_mask) return false;

  const auto image = reinterpret_cast<std::uintptr_t>(transaction.address(0));
  constexpr DWORD unwind_rva = 0xf00;
  constexpr std::array<std::byte, 4> empty_unwind_info{
      std::byte{1}, std::byte{0}, std::byte{0}, std::byte{0}};
  std::memcpy(
      reinterpret_cast<void*>(image + unwind_rva),
      empty_unwind_info.data(),
      empty_unwind_info.size());
  std::array<RUNTIME_FUNCTION, 9> functions{};
  for (std::size_t index = 0; index < functions.size(); ++index) {
    functions[index].BeginAddress = static_cast<DWORD>(0x800 + index * 64);
    functions[index].EndAddress =
        functions[index].BeginAddress + transaction.trampoline(index).byte_count;
    functions[index].UnwindData = unwind_rva;
  }
  if (RtlAddFunctionTable(
          functions.data(),
          static_cast<DWORD>(functions.size()),
          static_cast<DWORD64>(image)) == FALSE) {
    return false;
  }
  bool lookup_ok = true;
  for (const auto& function : functions) {
    DWORD64 discovered_base = 0;
    const auto* discovered = RtlLookupFunctionEntry(
        static_cast<DWORD64>(image + function.BeginAddress),
        &discovered_base,
        nullptr);
    if (discovered == nullptr || discovered_base != image ||
        discovered->BeginAddress != function.BeginAddress ||
        discovered->EndAddress != function.EndAddress ||
        discovered->UnwindData != function.UnwindData) {
      lookup_ok = false;
      break;
    }
  }
  const bool removed = RtlDeleteFunctionTable(functions.data()) != FALSE;
  return lookup_ok && removed;
}

bool selftest_typed_frames(std::byte* const page) noexcept {
  const auto image = reinterpret_cast<std::uintptr_t>(page);
  const auto manager = image + 0xa00;
  const auto engine = image + 0xb00;
  const auto record = image + 0xc00;
  const auto callback = image + 0xd00;
  std::memcpy(
      reinterpret_cast<void*>(manager + adapter::kManagerEngineOffset),
      &engine,
      sizeof(engine));
  constexpr std::int32_t order = 2;
  std::memcpy(
      reinterpret_cast<void*>(record + adapter::kBindOrderOffset),
      &order,
      sizeof(order));
  TypedBindMetadata metadata{};
  if (!extract_bind_metadata(
          image,
          manager,
          record,
          record + adapter::kBindRecordStride,
          callback,
          metadata) ||
      metadata.bind_order != order || metadata.callback_rva != 0xd00 ||
      !metadata.final_callback || metadata.engine_capability != engine) {
    return false;
  }
  if (extract_bind_metadata(
          image,
          manager,
          record + 1,
          record + adapter::kBindRecordStride,
          callback,
          metadata)) {
    return false;
  }

  const auto precompiled = image + 0xe00;
  const auto jit_info = image + 0xe80;
  std::memcpy(
      reinterpret_cast<void*>(manager + adapter::kManagerPrecompiledDataOffset),
      &precompiled,
      sizeof(precompiled));
  std::memcpy(
      reinterpret_cast<void*>(precompiled + adapter::kPrecompiledDataGuidOffset),
      target::kPrecompiledGuid.data(),
      target::kPrecompiledGuid.size());
  std::memcpy(
      reinterpret_cast<void*>(jit_info),
      target::kPrecompiledGuid.data(),
      target::kPrecompiledGuid.size());
  gore_as_capture_build_jit_v1 build_jit{};
  if (!extract_build_jit_metadata(
          manager, target::kBuildIdentifier, jit_info, build_jit) ||
      build_jit.struct_size != sizeof(build_jit) ||
      build_jit.shipping_cache_matches != 1 || build_jit.jit_info_present != 1 ||
      build_jit.jit_guid_matches != 1 || build_jit.jit_database_cleared != 0 ||
      build_jit.as_reference_debugging != 0 ||
      build_jit.fork_opcode_table_201_212_present != 1 ||
      build_jit.reference_debug_opcodes_emittable != 0 ||
      build_jit.resolve_object_ptr_callback_registered != 0) {
    return false;
  }
  reinterpret_cast<std::byte*>(jit_info)[0] ^= std::byte{1};
  const bool mismatch_detected =
      extract_build_jit_metadata(
          manager, target::kBuildIdentifier, jit_info, build_jit) &&
      build_jit.jit_guid_matches == 0 && build_jit.jit_database_cleared == 1;
  reinterpret_cast<std::byte*>(jit_info)[0] ^= std::byte{1};
  if (!mismatch_detected ||
      extract_build_jit_metadata(manager, target::kBuildIdentifier + 1, jit_info, build_jit)) {
    return false;
  }

  TypedFrontendMetadata frontend{};
  if (!extract_initial_compile_entry(manager, frontend) ||
      frontend.manager_capability != manager || frontend.engine_capability != engine) {
    return false;
  }
  const auto descriptor_array = image + 0xf00;
  const auto descriptor_data = image + 0xf20;
  constexpr std::int32_t descriptor_count = 2;
  std::memcpy(
      reinterpret_cast<void*>(descriptor_array), &descriptor_data, sizeof(descriptor_data));
  std::memcpy(
      reinterpret_cast<void*>(descriptor_array + 8),
      &descriptor_count,
      sizeof(descriptor_count));
  std::memcpy(
      reinterpret_cast<void*>(descriptor_array + 12),
      &descriptor_count,
      sizeof(descriptor_count));
  if (!extract_precompiled_descriptor_result(
          precompiled, descriptor_array, descriptor_array, frontend) ||
      frontend.boundary_object_capability != descriptor_array || frontend.item_count != 2 ||
      extract_precompiled_descriptor_result(
          precompiled, descriptor_array, descriptor_array + 8, frontend)) {
    return false;
  }
  const auto preprocessor = image + 0x900;
  if (!extract_preprocessor_constructed(preprocessor, frontend) ||
      frontend.boundary_object_capability != preprocessor) {
    return false;
  }

  const std::uint8_t success = 1;
  std::memcpy(
      reinterpret_cast<void*>(manager + adapter::kManagerInitialCompileSucceededOffset),
      &success,
      sizeof(success));
  if (!extract_initial_compile_return(manager, frontend) ||
      !frontend.initial_compile_succeeded) {
    return false;
  }
  constexpr std::uint8_t invalid_success = 2;
  std::memcpy(
      reinterpret_cast<void*>(manager + adapter::kManagerInitialCompileSucceededOffset),
      &invalid_success,
      sizeof(invalid_success));
  return !extract_initial_compile_return(manager, frontend);
}

bool selftest_thread_window(
    std::byte* const page,
    const std::size_t page_bytes = 4096) noexcept {
  if (page == nullptr || page_bytes == 0 ||
      reinterpret_cast<std::uintptr_t>(page) >
          std::numeric_limits<std::uintptr_t>::max() - page_bytes) {
    return false;
  }
  const HANDLE ready = CreateEventW(nullptr, TRUE, FALSE, nullptr);
  const HANDLE finish = CreateEventW(nullptr, TRUE, FALSE, nullptr);
  if (ready == nullptr || finish == nullptr) {
    if (ready != nullptr) (void)CloseHandle(ready);
    if (finish != nullptr) (void)CloseHandle(finish);
    return false;
  }
  bool worker_started = false;
  bool passed = false;
  try {
    std::thread worker([&] {
      (void)SetEvent(ready);
      (void)WaitForSingleObject(finish, INFINITE);
    });
    worker_started = true;
    if (WaitForSingleObject(ready, 5'000) == WAIT_OBJECT_0) {
      ThreadWindow window;
      const std::array ranges{
          std::pair{
              reinterpret_cast<std::uintptr_t>(page),
              reinterpret_cast<std::uintptr_t>(page) + page_bytes}};
      passed = window.acquire() && window.all_instruction_pointers_outside(ranges);
      window.release();
    }
    (void)SetEvent(finish);
    worker.join();
  } catch (...) {
    if (worker_started) (void)SetEvent(finish);
  }
  (void)CloseHandle(finish);
  (void)CloseHandle(ready);
  return passed;
}

bool selftest_registration_contracts() noexcept {
  if (!adapter::target_type_usage_selftest_v1() ||
      !adapter::target_registration_observer_selftest_v1() ||
      !adapter::target_frontend_observer_selftest_v1() ||
      !adapter::target_frontend_snapshot_builder_selftest_v1() ||
      !adapter::production_capture_phase_machine_selftest_v1() ||
      !adapter::production_capture_dispatcher_selftest_v1() ||
      !adapter::target_frontend_raw_materializer_selftest_v1()) {
    return false;
  }
  std::uint32_t kind_mask = 0;
  std::uint32_t previous_slot = 0;
  for (const auto& hook : registration::kPinnedRegistrationHooks) {
    if (hook.kind == 0 || hook.kind > 14 ||
        (kind_mask & (1u << (hook.kind - 1))) != 0 || hook.vtable_slot <= previous_slot ||
        hook.function_rva >= target::kPeSizeOfImage ||
        hook.function_rva > target::kPeSizeOfImage - hook.overwrite_bytes ||
        hook.overwrite_bytes < 14 || hook.source_prolog_bytes < hook.overwrite_bytes ||
        hook.argument_count == 0 || hook.unwind_operation_count == 0) {
      return false;
    }
    kind_mask |= 1u << (hook.kind - 1);
    previous_slot = hook.vtable_slot;
    const bool callable =
        hook.kind == GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1 ||
        hook.kind == GORE_AS_CAPTURE_REGISTRATION_OBJECT_METHOD_V1 ||
        hook.kind == GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1;
    if (((hook.contract_flags &
          (GORE_AS_CAPTURE_REGISTRATION_CONTRACT_AUXILIARY_TOKEN_V1 |
           GORE_AS_CAPTURE_REGISTRATION_CONTRACT_CALLER_DESCRIPTOR_V1)) != 0) !=
        callable) {
      return false;
    }
    for (std::size_t index = 0; index < hook.argument_count; ++index) {
      const auto& argument = hook.arguments[index];
      if (argument.source < GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1 ||
          argument.source > GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1 ||
          argument.semantic < GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1 ||
          argument.semantic > GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1) {
        return false;
      }
    }
  }
  return kind_mask == registration::kAllRegistrationHookMask;
}

struct GeneratedUnwindBlob final {
  std::array<std::byte, 64> bytes{};
  std::uint8_t byte_count{};
  std::uint8_t code_slot_count{};
};

bool build_generated_unwind_blob(
    const registration::RegistrationHookPoint& hook,
    GeneratedUnwindBlob& blob) noexcept {
  constexpr std::uint8_t kUwopAllocateLarge = 1;
  constexpr std::uint8_t kUwopSaveNonvolatile = 4;
  blob = {};
  blob.bytes[0] = std::byte{1};
  blob.bytes[1] = static_cast<std::byte>(hook.overwrite_bytes);
  std::size_t cursor = 4;
  std::uint8_t previous_code_offset = std::numeric_limits<std::uint8_t>::max();
  std::uint8_t slots = 0;
  for (std::size_t index = 0; index < hook.unwind_operation_count; ++index) {
    const auto& operation = hook.unwind[index];
    if (operation.code_offset == 0 || operation.code_offset > hook.overwrite_bytes ||
        operation.code_offset > previous_code_offset || cursor > blob.bytes.size() - 4) {
      return false;
    }
    previous_code_offset = operation.code_offset;
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
            kUwopSaveNonvolatile);
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
        blob.bytes[cursor++] = static_cast<std::byte>(kUwopAllocateLarge);
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
  blob.code_slot_count = slots;
  return true;
}

bool selftest_registration_unwind(
    RegistrationFixturePage& page,
    const FixtureRegistrationPatchTransaction& transaction) noexcept {
  constexpr std::uint32_t kUnwindBase = 0x5000;
  constexpr std::uint32_t kUnwindStride = 64;
  const auto image = reinterpret_cast<std::uintptr_t>(page.get());
  std::array<RUNTIME_FUNCTION, 14> functions{};
  for (std::size_t index = 0; index < functions.size(); ++index) {
    GeneratedUnwindBlob blob{};
    if (!build_generated_unwind_blob(registration::kPinnedRegistrationHooks[index], blob) ||
        blob.byte_count == 0 || blob.code_slot_count == 0) {
      return false;
    }
    std::memcpy(page.get() + kUnwindBase + index * kUnwindStride, blob.bytes.data(),
                blob.byte_count);
    functions[index].BeginAddress = static_cast<DWORD>(
        FixtureRegistrationPatchTransaction::kTrampolineBase +
        index * FixtureRegistrationPatchTransaction::kTrampolineStride);
    functions[index].EndAddress =
        functions[index].BeginAddress + transaction.trampoline(index).byte_count;
    functions[index].UnwindData =
        static_cast<DWORD>(kUnwindBase + index * kUnwindStride);
  }
  if (RtlAddFunctionTable(
          functions.data(), static_cast<DWORD>(functions.size()), image) == FALSE) {
    return false;
  }
  bool valid = true;
  for (const auto& function : functions) {
    DWORD64 discovered_base = 0;
    const auto* discovered = RtlLookupFunctionEntry(
        image + function.BeginAddress, &discovered_base, nullptr);
    if (discovered == nullptr || discovered_base != image ||
        discovered->BeginAddress != function.BeginAddress ||
        discovered->EndAddress != function.EndAddress ||
        discovered->UnwindData != function.UnwindData) {
      valid = false;
      break;
    }
  }
  const bool removed = RtlDeleteFunctionTable(functions.data()) != FALSE;
  return valid && removed;
}

bool assign_registration_source(
    RegistrationEntryFrame& frame,
    const std::uint8_t source,
    const std::uintptr_t value) noexcept {
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1:
      frame.rdx = value;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1:
      frame.r8 = value;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1:
      frame.r9 = value;
      return true;
    default:
      break;
  }
  std::uint32_t offset = 0;
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1:
      offset = 0x28;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1:
      offset = 0x30;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1:
      offset = 0x38;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1:
      offset = 0x40;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1:
      offset = 0x48;
      break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1:
      offset = 0x50;
      break;
    default:
      return false;
  }
  if (frame.original_rsp == 0 || !readable_range(frame.original_rsp + offset, sizeof(value))) {
    return false;
  }
  std::memcpy(reinterpret_cast<void*>(frame.original_rsp + offset), &value, sizeof(value));
  return true;
}

bool prepare_registration_fixture(
    std::byte* const page,
    const std::size_t hook_index,
    RegistrationEntryFrame& frame) noexcept {
  if (page == nullptr || hook_index >= registration::kPinnedRegistrationHooks.size()) {
    return false;
  }
  const auto engine = reinterpret_cast<std::uintptr_t>(page + 0x8000);
  const auto text = reinterpret_cast<std::uintptr_t>(page + 0x8100);
  const auto function_pointer = reinterpret_cast<std::uintptr_t>(page + 0x8200);
  const auto caller = reinterpret_cast<std::uintptr_t>(page + 0x8300);
  const auto auxiliary = reinterpret_cast<std::uintptr_t>(page + 0x8400);
  const auto stack = reinterpret_cast<std::uintptr_t>(page + 0x9000);
  constexpr char declaration[] = "void Example(int value)";
  std::memset(page + 0x8000, 0, 0x1100);
  std::memcpy(reinterpret_cast<void*>(text), declaration, sizeof(declaration));
  reinterpret_cast<std::byte*>(function_pointer)
      [adapter::layout_v23300::donor::function_pointer_descriptor_flag] = std::byte{2};
  std::memcpy(reinterpret_cast<void*>(caller), &auxiliary, sizeof(auxiliary));
  constexpr std::int32_t caller_type = 1;
  std::memcpy(
      reinterpret_cast<void*>(
          caller + adapter::layout_v23300::donor::function_caller_descriptor_type),
      &caller_type,
      sizeof(caller_type));
  frame = {};
  frame.rcx = engine;
  frame.original_rsp = stack;
  for (std::size_t index = 0;
       index < registration::kPinnedRegistrationHooks[hook_index].argument_count;
       ++index) {
    const auto& argument =
        registration::kPinnedRegistrationHooks[hook_index].arguments[index];
    std::uintptr_t value = 0;
    switch (argument.semantic) {
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1:
        value = text;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1:
        value = function_pointer;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1:
        value = 2;  // The fork-specific distinction that was lost by post-state snapshots.
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1:
        value = caller;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1:
        value = auxiliary;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1:
        value = 1;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1:
        value = 42;
        break;
      default:
        return false;
    }
    if (!assign_registration_source(frame, argument.source, value)) return false;
  }
  return true;
}

bool selftest_registration_extraction(RegistrationFixturePage& page) noexcept {
  for (std::size_t hook_index = 0;
       hook_index < registration::kPinnedRegistrationHooks.size();
       ++hook_index) {
    RegistrationEntryFrame frame{};
    ExtractedRegistrationEntry extracted{};
    if (!prepare_registration_fixture(page.get(), hook_index, frame) ||
        !extract_registration_entry(hook_index, frame, extracted) ||
        extracted.kind != registration::kPinnedRegistrationHooks[hook_index].kind ||
        extracted.engine_capability != frame.rcx ||
        extracted.argument_count !=
            registration::kPinnedRegistrationHooks[hook_index].argument_count) {
      return false;
    }
    for (std::size_t argument = 0; argument < extracted.argument_count; ++argument) {
      const auto semantic = extracted.arguments[argument].semantic;
      if (semantic == GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1 &&
          (extracted.arguments[argument].text_bytes == 0 ||
           extracted.arguments[argument]
                   .text[extracted.arguments[argument].text_bytes] != '\0')) {
        return false;
      }
      if (semantic == GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1 &&
          extracted.arguments[argument].scalar != 2) {
        return false;
      }
      if (semantic == GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1 &&
          extracted.arguments[argument].pointer_capability == 0) {
        return false;
      }
    }
  }

  RegistrationEntryFrame frame{};
  ExtractedRegistrationEntry ignored{};
  if (!prepare_registration_fixture(page.get(), 0, frame)) return false;
  frame.r9 = 9;
  if (extract_registration_entry(0, frame, ignored)) return false;
  if (!prepare_registration_fixture(page.get(), 0, frame)) return false;
  page.get()[0x8200 + adapter::layout_v23300::donor::function_pointer_descriptor_flag] =
      std::byte{0};
  if (extract_registration_entry(0, frame, ignored)) return false;
  if (!prepare_registration_fixture(page.get(), 0, frame)) return false;
  constexpr std::int32_t invalid_caller_type = 3;
  std::memcpy(
      page.get() + 0x8300 +
          adapter::layout_v23300::donor::function_caller_descriptor_type,
      &invalid_caller_type,
      sizeof(invalid_caller_type));
  if (extract_registration_entry(0, frame, ignored)) return false;
  if (!prepare_registration_fixture(page.get(), 0, frame)) return false;
  page.get()[0x8100] = std::byte{0xc0};
  page.get()[0x8101] = std::byte{0x80};
  page.get()[0x8102] = std::byte{0};
  if (extract_registration_entry(0, frame, ignored)) return false;
  if (!prepare_registration_fixture(page.get(), 3, frame)) return false;
  if (!assign_registration_source(
          frame, GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1, 2) ||
      extract_registration_entry(3, frame, ignored)) {
    return false;
  }
  return true;
}

bool selftest_registration_order() {
  RegistrationOrderTracker tracker(GetCurrentThreadId());
  std::uint64_t outer_sequence = 0;
  std::uint64_t inner_sequence = 0;
  if (!tracker.enter(GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1, 0x101, outer_sequence) ||
      !tracker.enter(GORE_AS_CAPTURE_REGISTRATION_ENUM_V1, 0x202, inner_sequence) ||
      outer_sequence != 0 || inner_sequence != 1 || tracker.depth() != 2) {
    return false;
  }
  std::uint64_t token = 0;
  std::uint64_t completed_sequence = 0;
  if (tracker.leave(
          GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1,
          7,
          token,
          completed_sequence)) {
    return false;
  }
  if (!tracker.leave(
          GORE_AS_CAPTURE_REGISTRATION_ENUM_V1, 17, token, completed_sequence) ||
      token != 0x202 || completed_sequence != inner_sequence || tracker.last_result() != 17 ||
      !tracker.leave(
          GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1,
          23,
          token,
          completed_sequence) ||
      token != 0x101 || completed_sequence != outer_sequence || tracker.last_result() != 23 ||
      tracker.depth() != 0) {
    return false;
  }
  bool wrong_thread_accepted = false;
  std::thread other([&] {
    std::uint64_t sequence = 0;
    wrong_thread_accepted =
        tracker.enter(GORE_AS_CAPTURE_REGISTRATION_TYPEDEF_V1, 0x303, sequence);
  });
  other.join();
  return !wrong_thread_accepted && tracker.depth() == 0;
}

bool selftest_registration_transaction(RegistrationFixturePage& page) {
  FixtureRegistrationPatchTransaction success(page.get());
  if (success.install() != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 ||
      !success.all_replaced()) {
    return false;
  }
  std::uint32_t wrong_thread = GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  std::thread other([&] { wrong_thread = success.uninstall(); });
  other.join();
  if (wrong_thread != GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1 ||
      success.uninstall() != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 ||
      !success.all_expected()) {
    return false;
  }

  FixtureRegistrationPatchTransaction drift(page.get());
  drift.address(6)[3] ^= std::byte{1};
  std::array<std::byte, RegistrationFixturePage::kBytes> before{};
  std::memcpy(before.data(), page.get(), before.size());
  if (drift.install() != GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1 ||
      std::memcmp(before.data(), page.get(), before.size()) != 0) {
    return false;
  }
  drift.address(6)[3] ^= std::byte{1};

  FixtureRegistrationPatchTransaction rollback(page.get());
  return rollback.install(7) == GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1 &&
         rollback.all_expected();
}

bool selftest_combined_unload_gate() noexcept {
  auto& state = instrumentation_state();
  {
    std::scoped_lock lock(state.mutex);
    if (state.installed || state.coordinator) return false;
    // Model the fail-closed interval in which instrumentation owns target state. A missing
    // coordinator is deliberately even less unloadable than an installed healthy coordinator.
    state.installed = true;
  }
  const auto blocked = gore_as_capture_bridge_prepare_unload_v1();
  {
    std::scoped_lock lock(state.mutex);
    state.installed = false;
  }
  return blocked == GORE_AS_CAPTURE_BRIDGE_BUSY_V1;
}

bool run_fixture_selftest(gore_as_capture_instrumentation_selftest_v1& result) {
  if (!selftest_combined_unload_gate()) return false;
  // The production transaction fixture reserves the exact active-target address range and a
  // rel32-near relay arena. Run it before the smaller synthetic pages fragment that address
  // window; this mirrors production preflight, which also allocates every relay before a write.
  const auto production_shim_stages =
      adapter::production_observer_shims_selftest_stages_v1();
  if (production_shim_stages == 0x1f) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_PRODUCTION_SHIMS_V1;
  }
  FixturePage page;
  RegistrationFixturePage registration_page;
  if (page.get() == nullptr || registration_page.get() == nullptr) return false;

  FixturePatchTransaction success(page.get());
  if (selftest_site_contracts()) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_SITE_CONTRACT_V1;
  }
  if (selftest_relocation_plans(success)) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_RELOCATION_V1;
  }
  if (selftest_unwind_plan(success)) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_UNWIND_PLAN_V1;
  }
  if (selftest_typed_frames(page.get())) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_TYPED_FRAMES_V1;
  }
  if (selftest_thread_window(page.get())) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_THREAD_WINDOW_V1;
  }
  if (adapter::public_registry_snapshot_selftest_v23300()) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_PUBLIC_REGISTRY_SNAPSHOT_V1;
  }
  if (selftest_registration_contracts() &&
      selftest_registration_extraction(registration_page)) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_REGISTRATION_CONTRACT_V1;
  }
  FixtureRegistrationPatchTransaction registration_unwind_fixture(
      registration_page.get());
  if (selftest_registration_unwind(
          registration_page, registration_unwind_fixture)) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_REGISTRATION_UNWIND_V1;
  }
  if (selftest_registration_order()) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_REGISTRATION_ORDER_V1;
  }
  if (adapter::target_final_state_selftest_v23300() &&
      adapter::target_capture_serializer_selftest_v1()) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_FINAL_STATE_EXTRACTOR_V1;
  }
  if (selftest_registration_transaction(registration_page) &&
      selftest_thread_window(
          registration_page.get(), RegistrationFixturePage::kBytes)) {
    result.reserved0 |= GORE_AS_CAPTURE_SELFTEST_REGISTRATION_TRANSACTION_V1;
  }
  result.installed_all_nine =
      success.install() == GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              success.all_replaced()
          ? 1u
          : 0u;
  result.unload_while_installed_refused =
      success.prepare_unload() == GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1 ? 1u : 0u;
  std::uint32_t wrong_thread = GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  std::thread other([&] { wrong_thread = success.uninstall(); });
  other.join();
  result.wrong_thread_refused =
      wrong_thread == GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1 &&
              success.installed()
          ? 1u
          : 0u;
  result.restored_all_nine =
      success.uninstall() == GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1 &&
              success.all_expected()
          ? 1u
          : 0u;

  FixturePatchTransaction drift(page.get());
  drift.address(4)[2] ^= std::byte{1};
  std::array<std::byte, 4096> drift_before{};
  std::memcpy(drift_before.data(), page.get(), drift_before.size());
  result.prolog_drift_refused_without_write =
      drift.install() == GORE_AS_CAPTURE_INSTRUMENTATION_PROLOG_DRIFT_V1 &&
              std::memcmp(page.get(), drift_before.data(), drift_before.size()) == 0
          ? 1u
          : 0u;
  drift.address(4)[2] ^= std::byte{1};

  FixturePatchTransaction rollback(page.get());
  result.injected_failure_rolled_back =
      rollback.install(4) == GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1 &&
              rollback.all_expected()
          ? 1u
          : 0u;

  RecordOrder exact;
  const std::array events{
      FixtureEvent::engine_property,
      FixtureEvent::bind_call,
      FixtureEvent::bind_return,
      FixtureEvent::registry_support,
      FixtureEvent::final_state,
      FixtureEvent::build_identifier,
      FixtureEvent::static_jit,
      FixtureEvent::frontend_configs,
      FixtureEvent::initial_compile_enter,
      FixtureEvent::frontend_middle,
      FixtureEvent::initial_compile_return,
      FixtureEvent::seal,
  };
  result.record_order_exact =
      std::all_of(events.begin(), events.end(), [&](const FixtureEvent event) {
        return exact.append(event);
      })
          ? 1u
          : 0u;
  RecordOrder wrong_order;
  result.record_order_drift_refused =
      !wrong_order.append(FixtureEvent::initial_compile_return) ? 1u : 0u;

  return result.installed_all_nine != 0 && result.restored_all_nine != 0 &&
         result.prolog_drift_refused_without_write != 0 &&
         result.injected_failure_rolled_back != 0 && result.wrong_thread_refused != 0 &&
         result.unload_while_installed_refused != 0 && result.record_order_exact != 0 &&
         result.record_order_drift_refused != 0 &&
         result.reserved0 == GORE_AS_CAPTURE_SELFTEST_STATIC_RE_ALL_V1;
}

#endif

}  // namespace

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_instrumentation_query_v1(
    gore_as_capture_instrumentation_contract_v1* const contract_out) {
  if (contract_out == nullptr) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  gore_as_capture_instrumentation_contract_v1 contract{};
  contract.struct_size = sizeof(contract);
  contract.abi_version = GORE_AS_CAPTURE_INSTRUMENTATION_ABI_V1;
  contract.steam_build_id = target::kSteamBuildId;
  contract.pe_size_of_image = target::kPeSizeOfImage;
  contract.codeview_age = target::kCodeViewAge;
  std::memcpy(
      contract.codeview_guid_rsds,
      target::kCodeViewGuidRsds.data(),
      sizeof(contract.codeview_guid_rsds));
  contract.hook_table_version = target::kHookTableVersion;
  contract.hook_point_count = static_cast<std::uint32_t>(target::kPinnedHookTable.size());
  contract.hook_table_fingerprint = target::kPinnedHookTableFingerprint;
  contract.prolog_table_fingerprint = adapter::kPinnedPrologTableFingerprint;
  contract.statically_extractable_hook_mask = adapter::kStaticallyExtractableHookMask;
  contract.unresolved_hook_mask = adapter::kUnresolvedHookMask;
  contract.production_installable = kTestFixtureOnly == 0 ? 1u : 0u;
  contract.test_fixture_only = kTestFixtureOnly;
  *contract_out = contract;
  return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_site_contract_v1(
    const std::uint32_t index,
    gore_as_capture_instrumentation_site_contract_v1* const contract_out) {
  if (contract_out == nullptr || index >= adapter::kPinnedInstructionSpans.size()) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  }
  const auto& span = adapter::kPinnedInstructionSpans[index];
  const auto& source = adapter::kStaticSiteContracts[index];
  gore_as_capture_instrumentation_site_contract_v1 contract{};
  contract.struct_size = sizeof(contract);
  contract.index = index;
  contract.hook_kind = static_cast<std::uint32_t>(span.kind);
  contract.observation_rva = span.observation_rva;
  contract.patch_anchor_rva = span.patch_anchor_rva;
  contract.overwrite_bytes = span.byte_count;
  contract.transfer_kind = source.transfer_kind;
  contract.continuation_rva = span.patch_anchor_rva + span.byte_count;
  contract.frame_kind = source.frame_kind;
  contract.register_read_mask = source.register_read_mask;
  contract.manager_offset = source.manager_offset;
  contract.engine_offset = source.engine_offset;
  contract.result_offset = source.result_offset;
  contract.record_stride = source.record_stride;
  contract.direct_callee_rva = source.direct_callee_rva;
  *contract_out = contract;
  return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_registration_hook_set_v1(
    gore_as_capture_registration_hook_set_v1* const contract_out) {
  if (contract_out == nullptr) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  gore_as_capture_registration_hook_set_v1 contract{};
  contract.struct_size = sizeof(contract);
  contract.contract_version = registration::kContractVersion;
  contract.hook_count =
      static_cast<std::uint32_t>(registration::kPinnedRegistrationHooks.size());
  contract.engine_vtable_rva = registration::kEngineVtableRva;
  contract.table_fingerprint = registration::kRegistrationTableFingerprint;
  contract.prolog_fingerprint = registration::kRegistrationPrologFingerprint;
  contract.statically_closed_hook_mask = registration::kAllRegistrationHookMask;
  contract.unresolved_hook_mask = 0;
  contract.production_installable = kTestFixtureOnly == 0 ? 1u : 0u;
  *contract_out = contract;
  return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_query_registration_site_v1(
    const std::uint32_t index,
    gore_as_capture_registration_site_contract_v1* const contract_out) {
  if (contract_out == nullptr || index >= registration::kPinnedRegistrationHooks.size()) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  }
  const auto& source = registration::kPinnedRegistrationHooks[index];
  gore_as_capture_registration_site_contract_v1 contract{};
  contract.struct_size = sizeof(contract);
  contract.index = index;
  contract.registration_kind = source.kind;
  contract.vtable_slot = source.vtable_slot;
  contract.function_rva = source.function_rva;
  contract.overwrite_bytes = source.overwrite_bytes;
  contract.continuation_rva = source.function_rva + source.overwrite_bytes;
  contract.generated_unwind_prolog_bytes = source.overwrite_bytes;
  contract.generated_unwind_operation_count = source.unwind_operation_count;
  contract.argument_count = source.argument_count;
  contract.return_source = GORE_AS_CAPTURE_REGISTRATION_RETURN_EAX_I32_V1;
  contract.contract_flags = source.contract_flags;
  contract.source_unwind_info_rva = source.source_unwind_info_rva;
  contract.source_prolog_bytes = source.source_prolog_bytes;
  for (std::size_t byte = 0; byte < source.expected.size(); ++byte) {
    contract.expected_prolog[byte] =
        std::to_integer<std::uint8_t>(source.expected[byte]);
  }
  for (std::size_t argument = 0; argument < source.arguments.size(); ++argument) {
    contract.argument_sources[argument] = source.arguments[argument].source;
    contract.argument_semantics[argument] = source.arguments[argument].semantic;
  }
  *contract_out = contract;
  return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_validate_current_image_v1(
    const std::uintptr_t primary_image_base) {
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  (void)primary_image_base;
  return GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1;
#else
  return validate_current_image(primary_image_base);
#endif
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_diagnose_patch_preflight_v1(
    const std::uintptr_t primary_image_base,
    std::uint32_t* const detail_out) {
  if (primary_image_base == 0 || detail_out == nullptr) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  }
  *detail_out = static_cast<std::uint32_t>(adapter::ProductionPatchError::invalid_state);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  return GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1;
#else
  const auto target_status = validate_current_image(primary_image_base);
  if (target_status != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) return target_status;
  try {
    adapter::ProductionPatchCoordinator diagnostic;
    const auto result = diagnostic.preflight(
        primary_image_base,
        1,
        adapter::ProductionShimObserver{
            nullptr,
            [](void*, std::uint32_t, adapter::ProductionShimPhase,
               adapter::ProductionMachineFrame&) noexcept { return true; }});
    *detail_out = static_cast<std::uint32_t>(result);
    return result == adapter::ProductionPatchError::ok
               ? GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1
               : GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
#endif
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_instrumentation_install_v1(
    const std::uint64_t session_id,
    const std::uintptr_t primary_image_base) {
  if (session_id == 0 || primary_image_base == 0) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  }
  try {
    auto& state = instrumentation_state();
    std::scoped_lock lock(state.mutex);
    if (state.installed) return GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1;
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
    return GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1;
#else
    const auto target_status = validate_current_image(primary_image_base);
    if (target_status != GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1) return target_status;
    auto coordinator = std::make_unique<adapter::ProductionCaptureCoordinator>();
    auto status = coordinator->preflight(
        session_id, primary_image_base, adapter::production_bridge_sink_v1());
    if (status != adapter::ProductionCaptureCoordinatorError::ok) {
      if (coordinator->recovery_required()) {
        state.coordinator = std::move(coordinator);
        state.installed = true;
        state.session_id = session_id;
        state.owner_thread = GetCurrentThreadId();
      }
      return instrumentation_error(status);
    }
    status = coordinator->install();
    if (status != adapter::ProductionCaptureCoordinatorError::ok) {
      if (coordinator->recovery_required()) {
        state.coordinator = std::move(coordinator);
        state.installed = true;
        state.session_id = session_id;
        state.owner_thread = GetCurrentThreadId();
      }
      return instrumentation_error(status);
    }
    state.coordinator = std::move(coordinator);
    state.installed = true;
    state.session_id = session_id;
    state.owner_thread = GetCurrentThreadId();
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
#endif
  } catch (...) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL gore_as_capture_instrumentation_uninstall_v1(
    const std::uint64_t session_id) {
  if (session_id == 0) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  try {
    auto& state = instrumentation_state();
    std::scoped_lock lock(state.mutex);
    if (!state.installed || state.session_id != session_id) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    }
    if (state.owner_thread != GetCurrentThreadId()) {
      return GORE_AS_CAPTURE_INSTRUMENTATION_WRONG_THREAD_V1;
    }
    if (!state.coordinator) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    const auto status = state.coordinator->uninstall();
    if (status != adapter::ProductionCaptureCoordinatorError::ok) {
      return instrumentation_error(status);
    }
    state.coordinator.reset();
    state.installed = false;
    state.session_id = 0;
    state.owner_thread = 0;
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_prepare_unload_v1() {
  try {
    auto& state = instrumentation_state();
    std::scoped_lock lock(state.mutex);
    if (!state.installed) return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
    if (!state.coordinator) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    return state.coordinator->prepare_unload() ==
                   adapter::ProductionCaptureCoordinatorError::ok
               ? GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1
               : GORE_AS_CAPTURE_INSTRUMENTATION_BUSY_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_observe_set_engine_property_v1(
    const std::uint64_t session_id,
    const std::uint32_t property_id_from_edx,
    const std::uint64_t value_from_r8) {
  if (property_id_from_edx == 0 || property_id_from_edx > 34) {
    return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
  return gore_as_capture_bridge_append_engine_property_v1(
      session_id,
      property_id_from_edx,
      value_from_r8,
      target::kRvaSetEngineProperty);
}

extern "C" std::uint32_t GORE_AS_CAPTURE_CALL
gore_as_capture_instrumentation_synthetic_selftest_v1(
    gore_as_capture_instrumentation_selftest_v1* const result_out) {
  if (result_out == nullptr) return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_ARGUMENT_V1;
  gore_as_capture_instrumentation_selftest_v1 result{};
  result.struct_size = sizeof(result);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  try {
    if (!run_fixture_selftest(result)) {
      *result_out = result;
      return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
    }
    *result_out = result;
    return GORE_AS_CAPTURE_INSTRUMENTATION_OK_V1;
  } catch (const std::bad_alloc&) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_PATCH_FAILED_V1;
  } catch (...) {
    return GORE_AS_CAPTURE_INSTRUMENTATION_INVALID_STATE_V1;
  }
#else
  *result_out = result;
  return GORE_AS_CAPTURE_INSTRUMENTATION_TEST_ONLY_V1;
#endif
}
