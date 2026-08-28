#include "gore_as_standalone/protocol.hpp"
#include "gore_as_standalone/sidecar_compile.hpp"

#include <algorithm>
#include <iostream>
#include <string>
#include <string_view>

namespace {

namespace wire = gore::as::standalone::protocol;

std::string json_escape(const std::string_view text) {
    std::string escaped;
    escaped.reserve(text.size());
    constexpr char hex[] = "0123456789abcdef";
    for (const unsigned char ch : text) {
        switch (ch) {
        case '"': escaped += "\\\""; break;
        case '\\': escaped += "\\\\"; break;
        case '\b': escaped += "\\b"; break;
        case '\f': escaped += "\\f"; break;
        case '\n': escaped += "\\n"; break;
        case '\r': escaped += "\\r"; break;
        case '\t': escaped += "\\t"; break;
        default:
            if (ch < 0x20U) {
                escaped += "\\u00";
                escaped += hex[(ch >> 4U) & 0x0fU];
                escaped += hex[ch & 0x0fU];
            } else escaped += static_cast<char>(ch);
        }
    }
    return escaped;
}

int emit_failure(
    const wire::ExitCode exit_code,
    const std::string_view code,
    const std::string_view message) {
    const auto bounded = message.substr(0U, std::min(message.size(), wire::kMaxDiagnosticMessageBytes));
    std::cout << "{\"response_version\":" << wire::kResponseProtocolVersion
        << ",\"ok\":false,\"failure_kind\":\"rejected\",\"diagnostics\":[{\"severity\":\"error\",\"code\":\""
        << json_escape(code) << "\",\"message\":\"" << json_escape(bounded) << "\"}]}\n";
    return static_cast<int>(exit_code);
}

void print_capabilities() {
    std::cout
        << "{\"backend\":\"gore-as-standalone-compiler\",\"backend_version\":\""
        << wire::kBackendVersion
        << "\",\"compatibility_id\":\"" << wire::kCompatibilityId
        << "\",\"request_version\":" << wire::kRequestProtocolVersionV2
        << ",\"request_versions\":[" << wire::kRequestProtocolVersionV1
        << ',' << wire::kRequestProtocolVersionV2 << ']'
        << ",\"qualification\":{\"available\":true,\"request_version\":"
        << wire::kQualificationProtocolVersionV3
        << ",\"requires_qualified_profile\":false,\"caller_witnesses\":false}"
        << ",\"response_version\":" << wire::kResponseProtocolVersion
        << ",\"transport\":{\"kind\":\"file\",\"encoding\":\"utf-8-json\"}"
        << ",\"core\":{\"available\":true,\"version\":\"" << wire::kCoreVersion
        << "\",\"dialect\":\"" << wire::kCoreDialect
        << "\",\"unreangel_revision\":\"" << wire::kUnreangelRevision << "\"}"
        << ",\"compile\":{\"available\":true,\"requires_qualified_profile\":true,"
           "\"requires_unreal_runtime\":false,\"requires_game_dll\":false}"
        << ",\"limits\":{\"request_bytes\":" << wire::kMaxRequestBytes
        << ",\"response_bytes\":" << wire::kMaxResponseBytes
        << ",\"diagnostics\":" << wire::kMaxDiagnostics
        << ",\"diagnostic_message_bytes\":" << wire::kMaxDiagnosticMessageBytes
        << ",\"json_nesting\":" << wire::kMaxJsonNestingDepth
        << ",\"source_files\":" << wire::kMaxSourceFiles
        << ",\"source_file_bytes\":" << wire::kMaxSourceFileBytes
        << ",\"aggregate_source_bytes\":" << wire::kMaxAggregateSourceBytes
        << "}}\n";
}

} // namespace

int wmain(const int argc, wchar_t* argv[]) {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << "gore-as-standalone-compiler " << wire::kBackendVersion << '\n';
        return static_cast<int>(wire::ExitCode::success);
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--capabilities") {
        print_capabilities();
        return static_cast<int>(wire::ExitCode::success);
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"compile" &&
        std::wstring_view(argv[2]) == L"--request") {
        auto result = gore::as::standalone::compile_sidecar_request(argv[3]);
        std::cout << result.response_json;
        return static_cast<int>(result.exit_code);
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"qualify" &&
        std::wstring_view(argv[2]) == L"--request") {
        auto result = gore::as::standalone::compile_sidecar_request(argv[3], true);
        std::cout << result.response_json;
        return static_cast<int>(result.exit_code);
    }
    return emit_failure(
        wire::ExitCode::usage,
        "GORE_AS_ARGUMENTS_INVALID",
        "usage: gore-as-standalone-compiler --version | --capabilities | compile --request <file> | qualify --request <file>");
}
