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

enum class CaptureTargetGeneration : std::uint32_t {
  build_24539464 = 24'539'464,
  build_24878692 = 24'878'692,
};

struct CaptureTarget final {
  CaptureTargetGeneration generation{};
  std::uint32_t steam_app_id{};
  std::uint64_t steam_build_id{};
  std::uint32_t angelscript_version{};
  std::uint64_t executable_bytes{};
  std::uint32_t pe_size_of_image{};
  std::array<std::byte, 32> executable_sha256{};
  std::array<std::byte, 16> codeview_guid_rsds{};
  std::uint32_t codeview_age{};
  std::uint32_t build_identifier{};
  std::array<std::byte, 16> precompiled_guid{};
  std::uint32_t rva_set_engine_property{};
  std::uint32_t rva_bind_callback_call{};
  std::uint32_t rva_bind_callback_return{};
  std::uint32_t rva_get_build_identifier{};
  std::uint32_t rva_get_static_jit_info{};
  std::uint32_t rva_initial_compile_enter{};
  std::uint32_t rva_precompiled_descriptors_requested{};
  std::uint32_t rva_preprocessor_constructed{};
  std::uint32_t rva_initial_compile_return{};
};

inline constexpr CaptureTarget kCaptureTarget24539464{
    CaptureTargetGeneration::build_24539464,
    1'297'900,
    24'539'464,
    23'300,
    171'784'704,
    0x0a7e4000,
    {std::byte{0xc7}, std::byte{0x1c}, std::byte{0x04}, std::byte{0xdd},
     std::byte{0x86}, std::byte{0xe1}, std::byte{0x1e}, std::byte{0x3e},
     std::byte{0x94}, std::byte{0x48}, std::byte{0x3e}, std::byte{0xa0},
     std::byte{0x2c}, std::byte{0x26}, std::byte{0xc6}, std::byte{0x12},
     std::byte{0xb6}, std::byte{0x24}, std::byte{0x3c}, std::byte{0x14},
     std::byte{0x7f}, std::byte{0x6d}, std::byte{0x83}, std::byte{0x97},
     std::byte{0x32}, std::byte{0x33}, std::byte{0xb3}, std::byte{0xc8},
     std::byte{0xdd}, std::byte{0xc5}, std::byte{0xde}, std::byte{0x25}},
    {std::byte{0xbd}, std::byte{0x83}, std::byte{0x0b}, std::byte{0xcf},
     std::byte{0x23}, std::byte{0xe0}, std::byte{0x1b}, std::byte{0x06},
     std::byte{0x21}, std::byte{0x00}, std::byte{0x0f}, std::byte{0x0f},
     std::byte{0xcc}, std::byte{0xf8}, std::byte{0x71}, std::byte{0xd2}},
    1,
    0x9e377abe,
    {std::byte{0xbe}, std::byte{0x78}, std::byte{0xfe}, std::byte{0x0a},
     std::byte{0x46}, std::byte{0xac}, std::byte{0x66}, std::byte{0x43},
     std::byte{0x96}, std::byte{0x85}, std::byte{0x97}, std::byte{0xe8},
     std::byte{0x5c}, std::byte{0x7e}, std::byte{0x5b}, std::byte{0x3f}},
    0x047a50f0,
    0x046856fb,
    0x046856fd,
    0x048d3230,
    0x048d0f60,
    0x04684210,
    0x046842d0,
    0x0468435d,
    0x04685a46,
};

inline constexpr CaptureTarget kCaptureTarget24878692{
    CaptureTargetGeneration::build_24878692,
    1'297'900,
    24'878'692,
    23'300,
    171'792'384,
    0x0a7e5000,
    {std::byte{0x82}, std::byte{0x4f}, std::byte{0xbc}, std::byte{0x94},
     std::byte{0xf2}, std::byte{0xac}, std::byte{0x7f}, std::byte{0x45},
     std::byte{0x92}, std::byte{0x7a}, std::byte{0x07}, std::byte{0x54},
     std::byte{0x60}, std::byte{0x56}, std::byte{0x66}, std::byte{0xc3},
     std::byte{0x7a}, std::byte{0xf8}, std::byte{0x62}, std::byte{0xd6},
     std::byte{0x61}, std::byte{0x56}, std::byte{0xa1}, std::byte{0x5f},
     std::byte{0x8b}, std::byte{0xf6}, std::byte{0x81}, std::byte{0x37},
     std::byte{0x59}, std::byte{0xd9}, std::byte{0xe8}, std::byte{0xe0}},
    {std::byte{0xda}, std::byte{0x4a}, std::byte{0xca}, std::byte{0xc2},
     std::byte{0x78}, std::byte{0x48}, std::byte{0x63}, std::byte{0xd9},
     std::byte{0xe5}, std::byte{0x67}, std::byte{0x71}, std::byte{0x7d},
     std::byte{0xc2}, std::byte{0xc4}, std::byte{0x83}, std::byte{0xa2}},
    1,
    0x9e377abe,
    {std::byte{0x78}, std::byte{0x35}, std::byte{0xbc}, std::byte{0xc0},
     std::byte{0x9c}, std::byte{0x5e}, std::byte{0xee}, std::byte{0x48},
     std::byte{0x8d}, std::byte{0x72}, std::byte{0xcb}, std::byte{0x5f},
     std::byte{0xfb}, std::byte{0x0f}, std::byte{0xb0}, std::byte{0xc3}},
    0x047a50b0,
    0x046856bb,
    0x046856bd,
    0x048d31f0,
    0x048d0f20,
    0x046841d0,
    0x04684290,
    0x0468431d,
    0x04685a06,
};

inline constexpr std::array<CaptureTarget, 2> kSupportedCaptureTargets{
    kCaptureTarget24539464,
    kCaptureTarget24878692,
};

// The production bridge/controller intentionally instruments only the newest authenticated
// generation. Offline decoders retain the historical descriptor above.
inline constexpr const CaptureTarget& kCaptureTarget = kCaptureTarget24878692;
inline constexpr std::uint32_t kSteamAppId = kCaptureTarget.steam_app_id;
inline constexpr std::uint64_t kSteamBuildId = kCaptureTarget.steam_build_id;
inline constexpr std::uint32_t kAngelScriptVersion = kCaptureTarget.angelscript_version;
inline constexpr std::uint64_t kExecutableBytes = kCaptureTarget.executable_bytes;
inline constexpr std::uint32_t kPeSizeOfImage = kCaptureTarget.pe_size_of_image;
inline constexpr const auto& kExecutableSha256 = kCaptureTarget.executable_sha256;
inline constexpr const auto& kCodeViewGuidRsds = kCaptureTarget.codeview_guid_rsds;
inline constexpr std::uint32_t kCodeViewAge = kCaptureTarget.codeview_age;
inline constexpr std::uint32_t kBuildIdentifier = kCaptureTarget.build_identifier;
inline constexpr const auto& kPrecompiledGuid = kCaptureTarget.precompiled_guid;
inline constexpr std::uint32_t kRvaSetEngineProperty =
    kCaptureTarget.rva_set_engine_property;
inline constexpr std::uint32_t kRvaBindCallbackCall =
    kCaptureTarget.rva_bind_callback_call;
inline constexpr std::uint32_t kRvaBindCallbackReturn =
    kCaptureTarget.rva_bind_callback_return;
inline constexpr std::uint32_t kRvaGetBuildIdentifier =
    kCaptureTarget.rva_get_build_identifier;
inline constexpr std::uint32_t kRvaGetStaticJitInfo =
    kCaptureTarget.rva_get_static_jit_info;
inline constexpr std::uint32_t kRvaInitialCompileEnter =
    kCaptureTarget.rva_initial_compile_enter;
inline constexpr std::uint32_t kRvaPrecompiledDescriptorsRequested =
    kCaptureTarget.rva_precompiled_descriptors_requested;
inline constexpr std::uint32_t kRvaPreprocessorConstructed =
    kCaptureTarget.rva_preprocessor_constructed;
inline constexpr std::uint32_t kRvaInitialCompileReturn =
    kCaptureTarget.rva_initial_compile_return;

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
