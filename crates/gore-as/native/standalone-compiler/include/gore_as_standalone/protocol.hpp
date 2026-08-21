#pragma once

#include <cstddef>
#include <cstdint>

namespace gore::as::standalone::protocol {

inline constexpr std::uint32_t kRequestProtocolVersion = 1;
inline constexpr std::uint32_t kResponseProtocolVersion = 1;
inline constexpr char kBackendVersion[] = "0.1.0-dev";
inline constexpr char kCoreVersion[] = "2.33.0 WIP";
inline constexpr char kCoreDialect[] = "UNREANGEL-modified";
inline constexpr char kUnreangelRevision[] = "247954da5326ecc29724067da7b5880c352fe4ff";

// These limits are part of the native process boundary. Raising one is a
// protocol review, not an implementation detail.
inline constexpr std::size_t kMaxRequestBytes = 1U * 1024U * 1024U;
inline constexpr std::size_t kMaxResponseBytes = 64U * 1024U;
inline constexpr std::size_t kMaxDiagnostics = 64U;
inline constexpr std::size_t kMaxDiagnosticMessageBytes = 2U * 1024U;
inline constexpr std::size_t kMaxRequestPathUtf16Units = 32'767U;
inline constexpr std::size_t kMaxJsonNestingDepth = 32U;

// Reserved compile-model bounds. The current fail-closed stub never accepts
// source payloads, but the eventual engine must not silently widen them.
inline constexpr std::size_t kMaxSourceFiles = 4'096U;
inline constexpr std::size_t kMaxSourceFileBytes = 16U * 1024U * 1024U;
inline constexpr std::size_t kMaxAggregateSourceBytes = 256U * 1024U * 1024U;

enum class ExitCode : int {
    success = 0,
    usage = 64,
    data_error = 65,
    unavailable = 69,
    software = 70,
};

} // namespace gore::as::standalone::protocol
