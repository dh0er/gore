#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace gore_as_capture::v1 {

inline constexpr std::array<std::byte, 8> kCaptureMagic{
    std::byte{'G'}, std::byte{'O'}, std::byte{'R'}, std::byte{'A'},
    std::byte{'S'}, std::byte{'C'}, std::byte{'A'}, std::byte{'P'}};
inline constexpr std::array<std::byte, 8> kFooterMagic{
    std::byte{'G'}, std::byte{'O'}, std::byte{'R'}, std::byte{'E'},
    std::byte{'S'}, std::byte{'E'}, std::byte{'A'}, std::byte{'L'}};
inline constexpr std::array<std::byte, 27> kHashDomain{
    std::byte{'g'}, std::byte{'o'}, std::byte{'r'}, std::byte{'e'}, std::byte{'-'},
    std::byte{'a'}, std::byte{'s'}, std::byte{'-'}, std::byte{'r'}, std::byte{'u'},
    std::byte{'n'}, std::byte{'t'}, std::byte{'i'}, std::byte{'m'}, std::byte{'e'},
    std::byte{'-'}, std::byte{'c'}, std::byte{'a'}, std::byte{'p'}, std::byte{'t'},
    std::byte{'u'}, std::byte{'r'}, std::byte{'e'}, std::byte{'-'}, std::byte{'v'},
    std::byte{'1'}, std::byte{0}};

inline constexpr std::uint16_t kSchemaVersion = 1;
inline constexpr std::size_t kHeaderBytes = 112;
inline constexpr std::size_t kRecordHeaderBytes = 24;
inline constexpr std::size_t kFooterBytes = 64;
inline constexpr std::uint64_t kMaxCaptureBytes = 512ull * 1024ull * 1024ull;
inline constexpr std::uint64_t kMaxRecords = 2'000'000;
inline constexpr std::uint32_t kMaxPayloadBytes = 256u * 1024u * 1024u;

inline constexpr std::uint32_t kSteamAppId = 1'297'900;
inline constexpr std::uint64_t kSteamBuildId = 24'539'464;
inline constexpr std::uint32_t kAngelScriptVersion = 23'300;
inline constexpr std::uint64_t kExecutableBytes = 171'784'704;
inline constexpr std::uint32_t kPeSizeOfImage = 0x0a7e4000;
inline constexpr std::array<std::byte, 32> kExecutableSha256{
    std::byte{0xc7}, std::byte{0x1c}, std::byte{0x04}, std::byte{0xdd},
    std::byte{0x86}, std::byte{0xe1}, std::byte{0x1e}, std::byte{0x3e},
    std::byte{0x94}, std::byte{0x48}, std::byte{0x3e}, std::byte{0xa0},
    std::byte{0x2c}, std::byte{0x26}, std::byte{0xc6}, std::byte{0x12},
    std::byte{0xb6}, std::byte{0x24}, std::byte{0x3c}, std::byte{0x14},
    std::byte{0x7f}, std::byte{0x6d}, std::byte{0x83}, std::byte{0x97},
    std::byte{0x32}, std::byte{0x33}, std::byte{0xb3}, std::byte{0xc8},
    std::byte{0xdd}, std::byte{0xc5}, std::byte{0xde}, std::byte{0x25}};
inline constexpr std::array<std::byte, 16> kCodeViewGuidRsds{
    std::byte{0xbd}, std::byte{0x83}, std::byte{0x0b}, std::byte{0xcf},
    std::byte{0x23}, std::byte{0xe0}, std::byte{0x1b}, std::byte{0x06},
    std::byte{0x21}, std::byte{0x00}, std::byte{0x0f}, std::byte{0x0f},
    std::byte{0xcc}, std::byte{0xf8}, std::byte{0x71}, std::byte{0xd2}};
inline constexpr std::uint32_t kCodeViewAge = 1;
inline constexpr std::uint32_t kBuildIdentifier = 0x9e377abe;
inline constexpr std::array<std::byte, 16> kPrecompiledGuid{
    std::byte{0xbe}, std::byte{0x78}, std::byte{0xfe}, std::byte{0x0a},
    std::byte{0x46}, std::byte{0xac}, std::byte{0x66}, std::byte{0x43},
    std::byte{0x96}, std::byte{0x85}, std::byte{0x97}, std::byte{0xe8},
    std::byte{0x5c}, std::byte{0x7e}, std::byte{0x5b}, std::byte{0x3f}};

inline constexpr std::uint32_t kRvaSetEngineProperty = 0x047a50f0;
inline constexpr std::uint32_t kRvaBindCallbackCall = 0x046856fb;
inline constexpr std::uint32_t kRvaBindCallbackReturn = 0x046856fd;
inline constexpr std::uint32_t kRvaGetBuildIdentifier = 0x048d3230;
inline constexpr std::uint32_t kRvaGetStaticJitInfo = 0x048d0f60;
inline constexpr std::uint32_t kRvaInitialCompileEnter = 0x04684210;
inline constexpr std::uint32_t kRvaPrecompiledDescriptorsRequested = 0x046842d0;
inline constexpr std::uint32_t kRvaPreprocessorConstructed = 0x0468435d;
inline constexpr std::uint32_t kRvaInitialCompileReturn = 0x04685a46;

enum class RecordKind : std::uint16_t {
  engine_property = 1,
  pointer_token = 2,
  bind_callback = 3,
  registry_delta_json = 4,
  post_bind_mutation_json = 5,
  final_post_bind_state_json = 6,
  build_jit = 7,
  frontend_boundary = 8,
  frontend_config_json = 9,
  registry_support_json = 10,
};

enum class FrontendBoundaryKind : std::uint32_t {
  initial_compile_enter = 1,
  precompiled_descriptors_requested = 2,
  preprocessor_constructed = 3,
  initial_compile_return = 4,
};

struct RegistryCounts final {
  std::uint32_t types{};
  std::uint32_t functions{};
  std::uint32_t object_properties{};
  std::uint32_t global_properties{};
  std::uint32_t enum_values{};
  std::uint32_t funcdefs{};
  std::uint32_t typedefs{};
  std::uint32_t total_registrations{};
};

using Digest = std::array<std::byte, 32>;
using GuidBytes = std::array<std::byte, 16>;

}  // namespace gore_as_capture::v1
