#pragma once

#include "gore_as_standalone/module_preprocessor.hpp"
#include "gore_as_standalone/protocol.hpp"

#include <string>
#include <string_view>

namespace gore::as::standalone {

struct sidecar_compile_result {
    protocol::ExitCode exit_code = protocol::ExitCode::software;
    std::string response_json;
};

// Exact BuildID-24539464 ClassAnalyze callback semantics. Exposed so the
// reversed target behavior has a direct native regression test independent of
// capture replay and the file protocol.
void apply_target_class_analyze_v24539464(
    const preprocessed_class_description& description,
    std::string& generated_statics,
    bool& has_statics);

// Reads and validates the file-based Protocol-v1 request, loads every sealed
// input without following reparse points, compiles the mixed graph and creates
// the requested cache path exactly once. Failures never publish a partial
// output.
sidecar_compile_result compile_sidecar_request(
    std::wstring_view request_path,
    bool allow_qualification = false) noexcept;

} // namespace gore::as::standalone
