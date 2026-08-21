#include "gore_as_standalone/protocol.hpp"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <iostream>
#include <limits>
#include <string>
#include <string_view>

namespace {

namespace wire = gore::as::standalone::protocol;

class unique_handle final {
public:
    explicit unique_handle(HANDLE value) noexcept : value_(value) {}
    ~unique_handle() {
        if (value_ != INVALID_HANDLE_VALUE) {
            CloseHandle(value_);
        }
    }

    unique_handle(const unique_handle&) = delete;
    unique_handle& operator=(const unique_handle&) = delete;

    [[nodiscard]] HANDLE get() const noexcept { return value_; }
    [[nodiscard]] bool valid() const noexcept { return value_ != INVALID_HANDLE_VALUE; }

private:
    HANDLE value_;
};

[[nodiscard]] std::string json_escape(std::string_view text) {
    std::string escaped;
    escaped.reserve(text.size());
    constexpr char hex[] = "0123456789abcdef";

    for (const unsigned char ch : text) {
        switch (ch) {
        case '\"': escaped += "\\\""; break;
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
            } else {
                escaped += static_cast<char>(ch);
            }
            break;
        }
    }
    return escaped;
}

[[nodiscard]] int emit_failure(
    const wire::ExitCode exit_code,
    const std::string_view code,
    const std::string_view message) {
    const auto bounded_message = message.substr(
        0U, std::min(message.size(), wire::kMaxDiagnosticMessageBytes));
    std::string response =
        "{\"response_version\":" + std::to_string(wire::kResponseProtocolVersion) +
        ",\"ok\":false,\"diagnostics\":[{\"severity\":\"error\",\"code\":\"" +
        json_escape(code) + "\",\"message\":\"" + json_escape(bounded_message) + "\"}]}\n";

    if (response.size() > wire::kMaxResponseBytes) {
        response = "{\"response_version\":1,\"ok\":false,\"diagnostics\":[{\"severity\":\"error\",\"code\":\"GORE_AS_RESPONSE_LIMIT\",\"message\":\"diagnostic exceeded the response limit\"}]}\n";
    }
    std::cout << response;
    return static_cast<int>(exit_code);
}

[[nodiscard]] bool is_valid_utf8(const std::string_view text) noexcept {
    std::size_t index = 0U;
    while (index < text.size()) {
        const auto first = static_cast<unsigned char>(text[index]);
        if (first <= 0x7fU) {
            ++index;
            continue;
        }

        std::size_t continuation_count = 0U;
        std::uint32_t code_point = 0U;
        std::uint32_t minimum = 0U;
        if ((first & 0xe0U) == 0xc0U) {
            continuation_count = 1U;
            code_point = first & 0x1fU;
            minimum = 0x80U;
        } else if ((first & 0xf0U) == 0xe0U) {
            continuation_count = 2U;
            code_point = first & 0x0fU;
            minimum = 0x800U;
        } else if ((first & 0xf8U) == 0xf0U) {
            continuation_count = 3U;
            code_point = first & 0x07U;
            minimum = 0x10000U;
        } else {
            return false;
        }

        if (continuation_count > text.size() - index - 1U) {
            return false;
        }
        for (std::size_t offset = 1U; offset <= continuation_count; ++offset) {
            const auto next = static_cast<unsigned char>(text[index + offset]);
            if ((next & 0xc0U) != 0x80U) {
                return false;
            }
            code_point = (code_point << 6U) | (next & 0x3fU);
        }
        if (code_point < minimum || code_point > 0x10ffffU ||
            (code_point >= 0xd800U && code_point <= 0xdfffU)) {
            return false;
        }
        index += continuation_count + 1U;
    }
    return true;
}

[[nodiscard]] bool is_json_object_envelope(const std::string_view text) noexcept {
    constexpr std::string_view whitespace = " \t\r\n";
    const auto first = text.find_first_not_of(whitespace);
    const auto last = text.find_last_not_of(whitespace);
    return first != std::string_view::npos && last != std::string_view::npos &&
        text[first] == '{' && text[last] == '}';
}

[[nodiscard]] bool is_within_json_nesting_limit(const std::string_view text) noexcept {
    std::array<char, wire::kMaxJsonNestingDepth> delimiters{};
    std::size_t depth = 0U;
    bool inside_string = false;
    bool escaped = false;

    for (const char ch : text) {
        if (inside_string) {
            if (escaped) {
                escaped = false;
            } else if (ch == '\\') {
                escaped = true;
            } else if (ch == '"') {
                inside_string = false;
            }
            continue;
        }
        if (ch == '"') {
            inside_string = true;
        } else if (ch == '{' || ch == '[') {
            if (depth == delimiters.size()) {
                return false;
            }
            delimiters[depth++] = ch;
        } else if (ch == '}' || ch == ']') {
            if (depth == 0U) {
                return false;
            }
            const auto opener = delimiters[depth - 1U];
            if ((ch == '}' && opener != '{') || (ch == ']' && opener != '[')) {
                return false;
            }
            --depth;
        }
    }
    return depth == 0U && !inside_string;
}

[[nodiscard]] int compile_stub(const std::wstring_view request_path) {
    if (request_path.empty() || request_path.size() > wire::kMaxRequestPathUtf16Units) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_PATH_INVALID",
            "request path is empty or exceeds the protocol limit");
    }

    const std::wstring null_terminated_path(request_path);
    unique_handle request_file(CreateFileW(
        null_terminated_path.c_str(),
        GENERIC_READ,
        FILE_SHARE_READ,
        nullptr,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_SEQUENTIAL_SCAN,
        nullptr));
    if (!request_file.valid()) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_OPEN_FAILED",
            "request file could not be opened for bounded read-only access");
    }

    if (GetFileType(request_file.get()) != FILE_TYPE_DISK) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_FILE_TYPE_REJECTED",
            "request must be a regular disk file");
    }

    FILE_ATTRIBUTE_TAG_INFO attributes{};
    if (!GetFileInformationByHandleEx(
            request_file.get(), FileAttributeTagInfo, &attributes, sizeof(attributes))) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_METADATA_FAILED",
            "request file metadata could not be read");
    }
    if ((attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0U ||
        (attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0U) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_FILE_TYPE_REJECTED",
            "request must be a regular file and must not be a reparse point");
    }

    LARGE_INTEGER size{};
    if (!GetFileSizeEx(request_file.get(), &size) || size.QuadPart < 0) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_SIZE_FAILED",
            "request file size could not be read");
    }
    if (static_cast<unsigned long long>(size.QuadPart) > wire::kMaxRequestBytes) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_TOO_LARGE",
            "request exceeds the protocol byte limit");
    }

    std::string request;
    request.reserve(static_cast<std::size_t>(size.QuadPart));
    std::array<char, 64U * 1024U> buffer{};
    for (;;) {
        const auto remaining_with_sentinel = wire::kMaxRequestBytes - request.size() + 1U;
        const auto wanted = static_cast<DWORD>(std::min(buffer.size(), remaining_with_sentinel));
        DWORD bytes_read = 0U;
        if (!ReadFile(request_file.get(), buffer.data(), wanted, &bytes_read, nullptr)) {
            return emit_failure(
                wire::ExitCode::data_error,
                "GORE_AS_REQUEST_READ_FAILED",
                "request file could not be read completely");
        }
        if (bytes_read == 0U) {
            break;
        }
        request.append(buffer.data(), bytes_read);
        if (request.size() > wire::kMaxRequestBytes) {
            return emit_failure(
                wire::ExitCode::data_error,
                "GORE_AS_REQUEST_TOO_LARGE",
                "request grew beyond the protocol byte limit while being read");
        }
    }

    if (request.empty()) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_EMPTY",
            "request file is empty");
    }
    if (request.find('\0') != std::string::npos || !is_valid_utf8(request)) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_ENCODING_INVALID",
            "request must contain valid UTF-8 without NUL bytes");
    }
    if (!is_json_object_envelope(request)) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_ENVELOPE_INVALID",
            "request must use a JSON object envelope");
    }
    if (!is_within_json_nesting_limit(request)) {
        return emit_failure(
            wire::ExitCode::data_error,
            "GORE_AS_REQUEST_NESTING_INVALID",
            "request has mismatched delimiters, an unterminated string, or exceeds the JSON nesting limit");
    }

    // Deliberately no parsing, compilation, cache lookup, or output write occurs
    // before the extracted compiler engine and profile validation are present.
    return emit_failure(
        wire::ExitCode::unavailable,
        "GORE_AS_STANDALONE_ENGINE_UNAVAILABLE",
        "standalone AngelScript engine integration is not available in this build");
}

void print_capabilities() {
    std::cout
        << "{\"backend\":\"gore-as-standalone-compiler\",\"backend_version\":\""
        << wire::kBackendVersion
        << "\",\"request_version\":" << wire::kRequestProtocolVersion
        << ",\"response_version\":" << wire::kResponseProtocolVersion
        << ",\"transport\":{\"kind\":\"file\",\"encoding\":\"utf-8-json\"}"
        << ",\"core\":{\"available\":true,\"version\":\"" << wire::kCoreVersion
        << "\",\"dialect\":\"" << wire::kCoreDialect
        << "\",\"unreangel_revision\":\"" << wire::kUnreangelRevision << "\"}"
        << ",\"compile\":{\"available\":false,\"requires_unreal_runtime\":false,"
           "\"requires_game_dll\":false}"
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
        return compile_stub(argv[3]);
    }
    return emit_failure(
        wire::ExitCode::usage,
        "GORE_AS_ARGUMENTS_INVALID",
        "usage: gore-as-standalone-compiler --version | --capabilities | compile --request <file>");
}
