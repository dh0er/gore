#pragma once

#include <cstddef>
#include <cstdint>

namespace gore::as::standalone::protocol {

inline constexpr std::uint32_t kRequestProtocolVersionV1 = 1;
inline constexpr std::uint32_t kRequestProtocolVersionV2 = 2;
// Qualification-only FullGraph transport. This is deliberately not the product compile
// protocol recorded in QualifiedSidecarIdentityV1; it may load a typed unqualified profile and
// returns same-process frontend/build/invoke evidence for promotion.
inline constexpr std::uint32_t kQualificationProtocolVersionV3 = 3;
inline constexpr std::uint32_t kResponseProtocolVersion = 1;
inline constexpr char kBackendVersion[] = "0.1.0-dev";
inline constexpr char kCoreVersion[] = "2.33.0 WIP";
inline constexpr char kCoreDialect[] = "UNREANGEL-modified";
inline constexpr char kUnreangelRevision[] = "247954da5326ecc29724067da7b5880c352fe4ff";

// These limits are part of the native process boundary. Raising one is a
// protocol review, not an implementation detail.
inline constexpr std::size_t kMaxRequestBytes = 16U * 1024U * 1024U;
inline constexpr std::size_t kMaxResponseBytes = 64U * 1024U;
inline constexpr std::size_t kMaxDiagnostics = 64U;
inline constexpr std::size_t kMaxDiagnosticMessageBytes = 2U * 1024U;
inline constexpr std::size_t kMaxRequestPathUtf16Units = 32'767U;
inline constexpr std::size_t kMaxJsonNestingDepth = 32U;

// The current pristine graph has 7,308 modules. A complete graph replacement
// plus more than 100% reserve remains bounded at the process boundary.
inline constexpr std::size_t kMaxSourceFiles = 16'384U;
inline constexpr std::size_t kMaxSourceFileBytes = 16U * 1024U * 1024U;
inline constexpr std::size_t kMaxAggregateSourceBytes = 1024U * 1024U * 1024U;
inline constexpr std::size_t kMaxOverlayModules = 16'384U;
inline constexpr std::size_t kMaxModuleIdentityBytes = 4U * 1024U;

enum class ExitCode : int {
    success = 0,
    usage = 64,
    data_error = 65,
    unavailable = 69,
    software = 70,
};

} // namespace gore::as::standalone::protocol
