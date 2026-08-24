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
inline constexpr std::uint32_t kBindArrayRva = 0x09874fd0;
inline constexpr std::uint32_t kStaticJitInfoGlobalRva = 0x09d6c2e8;
inline constexpr std::uint32_t kPrecompiledDataGuidOffset = 0x28;
inline constexpr std::uint32_t kStaticJitGuidBytes = 16;
inline constexpr std::uint32_t kPrecompiledDescriptorStride = 16;
inline constexpr bool kAsReferenceDebugging = false;
inline constexpr bool kForkOpcodeTable201Through212Present = true;
inline constexpr bool kReferenceDebugOpcodesEmittable = false;
inline constexpr bool kResolveObjectPtrCallbackRegistered = false;
inline constexpr std::uint32_t kRvaPrecompiledRequestCallee = 0x048d4690;
inline constexpr std::uint32_t kRvaPreprocessorConstructor = 0x04885800;
inline constexpr std::uint16_t kTargetDllCharacteristics = 0x8160;
inline constexpr std::uint32_t kTargetGuardFlags = 0x100;

struct UnsafeInstallRange final {
  std::uint32_t begin_rva{};
  std::uint32_t end_rva{};
};

// Function bounds are address-driven static-analysis results for the pinned image. Installation
// and removal reject a suspended thread in any range whose control flow contains a patched site.
inline constexpr std::array<UnsafeInstallRange, 5> kUnsafeInstallRanges{{
    {0x04684210, 0x046847f1},  // InitialCompile
    {0x04685160, 0x04685c5c},  // Initialize_AnyThread / bind loop / return site
    {0x047a50f0, 0x047a537b},  // asCScriptEngine::SetEngineProperty
    {0x048d0f60, 0x048d0f68},  // FStaticJITCompiledInfo::Get
    {0x048d3230, 0x048d34b3},  // GetCurrentBuildIdentifier
}};

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
     0x046856f8,
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
     {std::byte{0x48}, std::byte{0x8b}, std::byte{0x05}, std::byte{0x81},
      std::byte{0xb3}, std::byte{0x49}, std::byte{0x05}, std::byte{0xc3}}},
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
    // call. Any future detour must relocate the whole instruction from 0x468435c; patching
    // 0x468435d directly would silently corrupt its relative target.
    {HookPointKind::preprocessor_constructed,
     kRvaPreprocessorConstructed,
     0x0468435c,
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

}  // namespace gore_as_capture::v1::instrumentation
