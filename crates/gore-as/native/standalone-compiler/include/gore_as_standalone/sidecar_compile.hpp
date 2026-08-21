#pragma once

#include "gore_as_standalone/protocol.hpp"

#include <string>
#include <string_view>

namespace gore::as::standalone {

struct sidecar_compile_result {
    protocol::ExitCode exit_code = protocol::ExitCode::software;
    std::string response_json;
};

// Reads and validates the file-based Protocol-v1 request, loads every sealed
// input without following reparse points, compiles the mixed graph and creates
// the requested cache path exactly once. Failures never publish a partial
// output.
sidecar_compile_result compile_sidecar_request(std::wstring_view request_path) noexcept;

} // namespace gore::as::standalone
