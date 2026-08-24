#pragma once

#include "gore_as_capture/format.hpp"

#include <array>
#include <cstddef>
#include <cstdint>

namespace gore_as_capture::v1 {

enum class HookPointKind : std::uint32_t {
  set_engine_property = 1,
  bind_callback_call = 2,
  bind_callback_return = 3,
  get_build_identifier = 4,
  get_static_jit_info = 5,
  initial_compile_enter = 6,
  precompiled_descriptors_requested = 7,
  preprocessor_constructed = 8,
  initial_compile_return = 9,
};

struct HookPoint final {
  HookPointKind kind{};
  std::uint32_t image_rva{};
};

inline constexpr std::uint32_t kHookTableVersion = 1;
inline constexpr std::array<HookPoint, 9> kPinnedHookTable{{
    {HookPointKind::set_engine_property, kRvaSetEngineProperty},
    {HookPointKind::bind_callback_call, kRvaBindCallbackCall},
    {HookPointKind::bind_callback_return, kRvaBindCallbackReturn},
    {HookPointKind::get_build_identifier, kRvaGetBuildIdentifier},
    {HookPointKind::get_static_jit_info, kRvaGetStaticJitInfo},
    {HookPointKind::initial_compile_enter, kRvaInitialCompileEnter},
    {HookPointKind::precompiled_descriptors_requested,
     kRvaPrecompiledDescriptorsRequested},
    {HookPointKind::preprocessor_constructed, kRvaPreprocessorConstructed},
    {HookPointKind::initial_compile_return, kRvaInitialCompileReturn},
}};

namespace detail {

consteval std::uint64_t hook_table_fingerprint() {
  // This is an ABI drift detector, not a cryptographic trust root. Product trust still comes
  // from the exact catalogued DLL byte image and the target seals checked by CaptureSession.
  std::uint64_t hash = 14695981039346656037ull;
  const auto append_u32 = [&hash](const std::uint32_t value) {
    for (unsigned shift = 0; shift < 32; shift += 8) {
      hash ^= static_cast<std::uint8_t>((value >> shift) & 0xffu);
      hash *= 1099511628211ull;
    }
  };
  append_u32(kHookTableVersion);
  for (const auto& point : kPinnedHookTable) {
    append_u32(static_cast<std::uint32_t>(point.kind));
    append_u32(point.image_rva);
  }
  return hash;
}

}  // namespace detail

inline constexpr std::uint64_t kPinnedHookTableFingerprint =
    detail::hook_table_fingerprint();

static_assert(kPinnedHookTable.front().image_rva != 0);
static_assert(kPinnedHookTable.size() == 9);

}  // namespace gore_as_capture::v1
