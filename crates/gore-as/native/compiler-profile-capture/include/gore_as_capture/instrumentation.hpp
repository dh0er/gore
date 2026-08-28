#pragma once

#include "gore_as_capture/hook_table.hpp"
#include "gore_as_capture/instrumentation.h"
#include "gore_as_capture/registration_hook_contract.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <span>

namespace gore_as_capture::v1::instrumentation {

inline constexpr std::uint32_t kNotApplicableOffset = 0xffff'ffffu;
inline constexpr std::uint32_t kManagerEngineOffset = 0x28;
inline constexpr std::uint32_t kManagerPrecompiledDataOffset = 0x460;
inline constexpr std::uint32_t kManagerStaticJitOffset = 0x468;
inline constexpr std::uint32_t kManagerInitialCompileSucceededOffset = 0x388;
inline constexpr std::uint32_t kBindRecordStride = 0x50;
inline constexpr std::uint32_t kBindOrderOffset = 0;
inline constexpr std::uint32_t kPrecompiledDataGuidOffset = 0x28;
inline constexpr std::uint32_t kStaticJitGuidBytes = 16;
inline constexpr std::uint32_t kPrecompiledDescriptorStride = 16;
inline constexpr bool kAsReferenceDebugging = false;
inline constexpr bool kForkOpcodeTable201Through212Present = true;
inline constexpr bool kReferenceDebugOpcodesEmittable = false;
inline constexpr bool kResolveObjectPtrCallbackRegistered = false;
struct UnsafeInstallRange final {
  std::uint32_t begin_rva{};
  std::uint32_t end_rva{};
};

struct InstrumentationTarget final {
  CaptureTargetGeneration generation{};
  std::uint32_t bind_array_rva{};
  std::uint32_t static_jit_info_global_rva{};
  std::uint32_t rva_precompiled_request_callee{};
  std::uint32_t rva_preprocessor_constructor{};
  std::uint32_t bind_callback_patch_anchor_rva{};
  std::uint32_t preprocessor_patch_anchor_rva{};
  std::array<std::byte, 8> get_static_jit_expected{};
  std::uint16_t dll_characteristics{};
  std::uint32_t guard_flags{};
  std::array<UnsafeInstallRange, 5> unsafe_install_ranges{};
};

inline constexpr InstrumentationTarget kInstrumentationTarget24539464{
    CaptureTargetGeneration::build_24539464,
    0x09874fd0,
    0x09d6c2e8,
    0x048d4690,
    0x04885800,
    0x046856f8,
    0x0468435c,
    {std::byte{0x48}, std::byte{0x8b}, std::byte{0x05}, std::byte{0x81},
     std::byte{0xb3}, std::byte{0x49}, std::byte{0x05}, std::byte{0xc3}},
    0x8160,
    0x100,
    {{{0x04684210, 0x046847f1},
      {0x04685160, 0x04685c5c},
      {0x047a50f0, 0x047a537b},
      {0x048d0f60, 0x048d0f68},
      {0x048d3230, 0x048d34b3}}},
};

inline constexpr InstrumentationTarget kInstrumentationTarget24878692{
    CaptureTargetGeneration::build_24878692,
    0x09875fd0,
    0x09d6d468,
    0x048d4650,
    0x048857c0,
    0x046856b8,
    0x0468431c,
    {std::byte{0x48}, std::byte{0x8b}, std::byte{0x05}, std::byte{0x41},
     std::byte{0xc5}, std::byte{0x49}, std::byte{0x05}, std::byte{0xc3}},
    0x8160,
    0x100,
    {{{0x046841d0, 0x046847b1},
      {0x04685120, 0x04685c1c},
      {0x047a50b0, 0x047a533b},
      {0x048d0f20, 0x048d0f28},
      {0x048d31f0, 0x048d3473}}},
};

inline constexpr const InstrumentationTarget& kInstrumentationTarget =
    kInstrumentationTarget24878692;
inline constexpr std::uint32_t kBindArrayRva = kInstrumentationTarget.bind_array_rva;
inline constexpr std::uint32_t kStaticJitInfoGlobalRva =
    kInstrumentationTarget.static_jit_info_global_rva;
inline constexpr std::uint32_t kRvaPrecompiledRequestCallee =
    kInstrumentationTarget.rva_precompiled_request_callee;
inline constexpr std::uint32_t kRvaPreprocessorConstructor =
    kInstrumentationTarget.rva_preprocessor_constructor;
inline constexpr std::uint16_t kTargetDllCharacteristics =
    kInstrumentationTarget.dll_characteristics;
inline constexpr std::uint32_t kTargetGuardFlags = kInstrumentationTarget.guard_flags;
static_assert(kInstrumentationTarget.generation == kCaptureTarget.generation);

// Function bounds are address-driven static-analysis results for the pinned image. Installation
// and removal reject a suspended thread in any range whose control flow contains a patched site.
inline constexpr auto kUnsafeInstallRanges = kInstrumentationTarget.unsafe_install_ranges;

struct StaticSiteContract final {
  std::uint32_t transfer_kind{};
  std::uint32_t frame_kind{};
  std::uint32_t register_read_mask{};
  std::uint32_t manager_offset{kNotApplicableOffset};
  std::uint32_t engine_offset{kNotApplicableOffset};
  std::uint32_t result_offset{kNotApplicableOffset};
  std::uint32_t record_stride{};
  std::uint32_t direct_callee_rva{};
};

struct PinnedInstructionSpan final {
  HookPointKind kind{};
  std::uint32_t observation_rva{};
  std::uint32_t patch_anchor_rva{};
  std::uint8_t observation_offset{};
  std::uint8_t byte_count{};
  std::array<std::byte, 16> expected{};
};

template <std::size_t Size>
consteval std::array<std::byte, 16> pinned_expected(
    const std::array<std::byte, Size>& source) {
  static_assert(Size <= 16);
  std::array<std::byte, 16> result{};
  for (std::size_t index = 0; index < source.size(); ++index) {
    result[index] = source[index];
  }
  return result;
}

inline constexpr std::array<PinnedInstructionSpan, 9> kPinnedInstructionSpans{{
    {HookPointKind::set_engine_property,
     kRvaSetEngineProperty,
     kRvaSetEngineProperty,
     0,
     14,
     {std::byte{0xff}, std::byte{0xca}, std::byte{0x83}, std::byte{0xfa},
      std::byte{0x21}, std::byte{0x0f}, std::byte{0x87}, std::byte{0x7a},
      std::byte{0x02}, std::byte{0x00}, std::byte{0x00}, std::byte{0x48},
      std::byte{0x63}, std::byte{0xc2}}},
    {HookPointKind::bind_callback_call,
     kRvaBindCallbackCall,
     kInstrumentationTarget.bind_callback_patch_anchor_rva,
     3,
     5,
     {std::byte{0x48}, std::byte{0x8b}, std::byte{0xc8}, std::byte{0xff},
      std::byte{0xd7}}},
    {HookPointKind::bind_callback_return,
     kRvaBindCallbackReturn,
     kRvaBindCallbackReturn,
     0,
     8,
     {std::byte{0x49}, std::byte{0x83}, std::byte{0xc7}, std::byte{0x50},
      std::byte{0x4d}, std::byte{0x8d}, std::byte{0x76}, std::byte{0x50}}},
    {HookPointKind::get_build_identifier,
     kRvaGetBuildIdentifier,
     kRvaGetBuildIdentifier,
     0,
     5,
     {std::byte{0x48}, std::byte{0x89}, std::byte{0x5c}, std::byte{0x24},
      std::byte{0x18}}},
    {HookPointKind::get_static_jit_info,
     kRvaGetStaticJitInfo,
     kRvaGetStaticJitInfo,
     0,
     8,
     pinned_expected(kInstrumentationTarget.get_static_jit_expected)},
    {HookPointKind::initial_compile_enter,
     kRvaInitialCompileEnter,
     kRvaInitialCompileEnter,
     0,
     12,
     {std::byte{0x4c}, std::byte{0x8b}, std::byte{0xdc}, std::byte{0x55},
      std::byte{0x53}, std::byte{0x49}, std::byte{0x8d}, std::byte{0xab},
      std::byte{0x98}, std::byte{0xfe}, std::byte{0xff}, std::byte{0xff}}},
    {HookPointKind::precompiled_descriptors_requested,
     kRvaPrecompiledDescriptorsRequested,
     kRvaPrecompiledDescriptorsRequested,
     0,
     5,
     {std::byte{0xe8}, std::byte{0xbb}, std::byte{0x03}, std::byte{0x25},
      std::byte{0x00}}},
    // The published semantic observation RVA is the first displacement byte of the containing
    // call. Any future detour must relocate the whole generation-selected call instruction;
    // the displacement observation directly would silently corrupt its relative target.
    {HookPointKind::preprocessor_constructed,
     kRvaPreprocessorConstructed,
     kInstrumentationTarget.preprocessor_patch_anchor_rva,
     1,
     5,
     {std::byte{0xe8}, std::byte{0x9f}, std::byte{0x14}, std::byte{0x20},
      std::byte{0x00}}},
    {HookPointKind::initial_compile_return,
     kRvaInitialCompileReturn,
     kRvaInitialCompileReturn,
     0,
     7,
     {std::byte{0x48}, std::byte{0x8b}, std::byte{0x8b}, std::byte{0x68},
      std::byte{0x04}, std::byte{0x00}, std::byte{0x00}}},
}};

inline constexpr std::array<StaticSiteContract, 9> kStaticSiteContracts{{
    {GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_SET_ENGINE_PROPERTY_V1,
     GORE_AS_CAPTURE_REGISTER_RCX_V1 | GORE_AS_CAPTURE_REGISTER_RDX_V1 |
         GORE_AS_CAPTURE_REGISTER_R8_V1},
    {GORE_AS_CAPTURE_TRANSFER_INLINE_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_BIND_CALL_V1,
     GORE_AS_CAPTURE_REGISTER_RAX_V1 | GORE_AS_CAPTURE_REGISTER_RCX_V1 |
         GORE_AS_CAPTURE_REGISTER_RBX_V1 | GORE_AS_CAPTURE_REGISTER_R12_V1 |
         GORE_AS_CAPTURE_REGISTER_R15_V1 | GORE_AS_CAPTURE_REGISTER_RDI_V1,
     0,
     kManagerEngineOffset,
     kBindOrderOffset,
     kBindRecordStride},
    {GORE_AS_CAPTURE_TRANSFER_INLINE_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_BIND_RETURN_V1,
     GORE_AS_CAPTURE_REGISTER_RBX_V1 | GORE_AS_CAPTURE_REGISTER_R12_V1 |
         GORE_AS_CAPTURE_REGISTER_R15_V1 | GORE_AS_CAPTURE_REGISTER_RDI_V1,
     0,
     kManagerEngineOffset,
     kBindOrderOffset,
     kBindRecordStride},
    {GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_BUILD_IDENTIFIER_V1,
     GORE_AS_CAPTURE_REGISTER_RAX_V1},
    {GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_STATIC_JIT_INFO_V1,
     GORE_AS_CAPTURE_REGISTER_RAX_V1,
     kNotApplicableOffset,
     kNotApplicableOffset,
     0},
    {GORE_AS_CAPTURE_TRANSFER_FUNCTION_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_INITIAL_COMPILE_ENTER_V1,
     GORE_AS_CAPTURE_REGISTER_RCX_V1,
     0,
     kManagerEngineOffset,
     kManagerInitialCompileSucceededOffset},
    {GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1,
     GORE_AS_CAPTURE_FRAME_PRECOMPILED_REQUEST_V1,
     GORE_AS_CAPTURE_REGISTER_RAX_V1 | GORE_AS_CAPTURE_REGISTER_RCX_V1 |
         GORE_AS_CAPTURE_REGISTER_RDX_V1,
     kNotApplicableOffset,
     kNotApplicableOffset,
     kNotApplicableOffset,
     0,
     kRvaPrecompiledRequestCallee},
    {GORE_AS_CAPTURE_TRANSFER_CALL_REWRITE_V1,
     GORE_AS_CAPTURE_FRAME_PREPROCESSOR_CONSTRUCTED_V1,
     GORE_AS_CAPTURE_REGISTER_RCX_V1,
     kNotApplicableOffset,
     kNotApplicableOffset,
     kNotApplicableOffset,
     0,
     kRvaPreprocessorConstructor},
    {GORE_AS_CAPTURE_TRANSFER_INLINE_JUMP_V1,
     GORE_AS_CAPTURE_FRAME_INITIAL_COMPILE_RETURN_V1,
     GORE_AS_CAPTURE_REGISTER_RBX_V1,
     0,
     kManagerEngineOffset,
     kManagerInitialCompileSucceededOffset},
}};

consteval std::uint64_t prolog_table_fingerprint() {
  std::uint64_t hash = 14695981039346656037ull;
  const auto append = [&hash](const std::uint8_t value) {
    hash ^= value;
    hash *= 1099511628211ull;
  };
  const auto append_u32 = [&append](const std::uint32_t value) {
    for (unsigned shift = 0; shift < 32; shift += 8) {
      append(static_cast<std::uint8_t>((value >> shift) & 0xffu));
    }
  };
  for (const auto& span : kPinnedInstructionSpans) {
    append_u32(static_cast<std::uint32_t>(span.kind));
    append_u32(span.observation_rva);
    append_u32(span.patch_anchor_rva);
    append(span.observation_offset);
    append(span.byte_count);
    for (std::size_t index = 0; index < span.byte_count; ++index) {
      append(std::to_integer<std::uint8_t>(span.expected[index]));
    }
  }
  return hash;
}

inline constexpr std::uint64_t kPinnedPrologTableFingerprint =
    prolog_table_fingerprint();
inline constexpr std::uint32_t kAllHookMask = (1u << kPinnedInstructionSpans.size()) - 1u;
inline constexpr std::uint32_t kStaticallyExtractableHookMask = kAllHookMask;
inline constexpr std::uint32_t kUnresolvedHookMask = 0;

static_assert(kPinnedInstructionSpans.size() == kPinnedHookTable.size());
static_assert(kStaticSiteContracts.size() == kPinnedHookTable.size());
static_assert([] {
  for (std::size_t index = 0; index < kPinnedHookTable.size(); ++index) {
    if (kPinnedInstructionSpans[index].kind != kPinnedHookTable[index].kind ||
        kPinnedInstructionSpans[index].observation_rva !=
            kPinnedHookTable[index].image_rva ||
        kPinnedInstructionSpans[index].observation_rva !=
            kPinnedInstructionSpans[index].patch_anchor_rva +
                kPinnedInstructionSpans[index].observation_offset ||
        kPinnedInstructionSpans[index].byte_count < 5) {
      return false;
    }
  }
  return true;
}());
static_assert(kPinnedInstructionSpans[7].observation_offset == 1);
static_assert([] {
  const auto& span = kPinnedInstructionSpans[4];
  if (span.kind != HookPointKind::get_static_jit_info ||
      span.byte_count != kInstrumentationTarget.get_static_jit_expected.size()) {
    return false;
  }
  for (std::size_t index = 0; index < span.byte_count; ++index) {
    if (span.expected[index] != kInstrumentationTarget.get_static_jit_expected[index]) {
      return false;
    }
  }
  return true;
}());

}  // namespace gore_as_capture::v1::instrumentation
