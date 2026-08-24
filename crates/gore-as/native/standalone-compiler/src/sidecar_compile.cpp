#include "gore_as_standalone/sidecar_compile.hpp"

#include "gore_as_standalone/frontend_compile.hpp"
#include "gore_as_standalone/json.hpp"
#include "gore_as_standalone/precompiled_data.hpp"
#include "gore_as_standalone/precompiled_engine.hpp"
#include "gore_as_standalone/profile_loader.hpp"
#include "gore_as_standalone/registry_profile.hpp"
#include "gore_as_standalone/sha256.hpp"

#include "AngelscriptManager.h"
#include "angelscript.h"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <cctype>
#include <cwctype>
#include <cstring>
#include <limits>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace gore::as::standalone {
namespace {

namespace wire = protocol;
namespace cache_wire = precompiled;

// There is intentionally no setter path in the standalone reconstruction. The donor only
// registers this callback in WITH_EDITOR && UE5, while the target profile is Shipping. If a
// future runtime adds registration, this constant and the qualification response must change
// together; Rust also requires the observed ResolveObjectPtr opcode count to remain exactly zero.
constexpr bool qualification_resolve_object_ptr_callback_registered = false;

constexpr std::uint64_t max_profile_blob_bytes = 512ULL * 1024ULL * 1024ULL;
constexpr std::uint64_t max_profile_aggregate_bytes = 1024ULL * 1024ULL * 1024ULL;
constexpr std::uint64_t max_base_cache_bytes = 512ULL * 1024ULL * 1024ULL;
constexpr std::uint64_t max_binds_cache_bytes = 128ULL * 1024ULL * 1024ULL;

class unique_handle final {
public:
    explicit unique_handle(const HANDLE value = INVALID_HANDLE_VALUE) noexcept : value_(value) {}
    ~unique_handle() { if (value_ != INVALID_HANDLE_VALUE) CloseHandle(value_); }
    unique_handle(unique_handle&& other) noexcept : value_(other.value_) { other.value_ = INVALID_HANDLE_VALUE; }
    unique_handle& operator=(unique_handle&& other) noexcept {
        if (this != &other) {
            if (value_ != INVALID_HANDLE_VALUE) CloseHandle(value_);
            value_ = other.value_; other.value_ = INVALID_HANDLE_VALUE;
        }
        return *this;
    }
    unique_handle(const unique_handle&) = delete;
    unique_handle& operator=(const unique_handle&) = delete;
    [[nodiscard]] HANDLE get() const noexcept { return value_; }
    [[nodiscard]] bool valid() const noexcept { return value_ != INVALID_HANDLE_VALUE; }
private:
    HANDLE value_;
};

struct engine_deleter {
    void operator()(asIScriptEngine* engine) const noexcept {
        if (engine != nullptr) engine->ShutDownAndRelease();
    }
};

using engine_ptr = std::unique_ptr<asIScriptEngine, engine_deleter>;

struct path_seal {
    std::string path_utf8;
    std::wstring path;
    std::uint64_t byte_len = 0U;
    sha256_digest sha256{};
};

struct source_file {
    std::string relative_path;
    std::uint64_t byte_len = 0U;
    sha256_digest sha256{};
};

struct overlay_module {
    std::uint32_t ordinal = 0U;
    preprocessor_source::operation operation = preprocessor_source::operation::add;
    std::string module_name;
    std::string relative_path;
};

enum class graph_change_operation {
    add,
    edit,
    remove,
};

struct graph_change {
    std::uint32_t ordinal = 0U;
    graph_change_operation operation = graph_change_operation::add;
    std::string module_name;
    std::string relative_path;
    std::uint64_t source_byte_len = 0U;
    sha256_digest source_sha256{};
    bool has_source = false;
};

struct final_module {
    std::uint32_t ordinal = 0U;
    std::string module_name;
    std::string relative_path;
};

struct qualification_request {
    std::string suite_id;
    sha256_digest corpus_sha256{};
    std::string case_id;
    std::string phase;
    std::string invoke_module;
    std::string invoke_declaration;
    sha256_digest request_sha256{};
};

struct compile_request {
    std::uint32_t request_version = wire::kRequestProtocolVersionV1;
    std::string manifest_path_utf8;
    std::wstring manifest_path;
    std::string profile_root_utf8;
    std::wstring profile_root;
    sha256_digest profile_sha256{};
    std::uint64_t steam_build_id = 0U;
    std::uint32_t depot_id = 0U;
    std::uint64_t depot_manifest_gid = 0U;
    std::string required_probe_suite_version;
    path_seal base_cache;
    path_seal binds_cache;
    std::string source_root_utf8;
    std::wstring source_root;
    std::vector<source_file> source_files;
    std::vector<overlay_module> overlays;
    std::vector<graph_change> changes;
    std::vector<final_module> final_manifest;
    std::string output_path_utf8;
    std::wstring output_path;
    bool qualification_mode = false;
    qualification_request qualification;
};

struct qualification_hook_capture {
    std::string subject_identity;
    std::vector<std::string> generated_declarations;
};

struct qualification_trace {
    bool class_analyze_bound = false;
    std::vector<qualification_hook_capture> class_analyze;
    bool process_chunks_bound = false;
    std::vector<qualification_hook_capture> process_chunks;
    bool post_process_code_bound = false;
    std::vector<qualification_hook_capture> post_process;
    std::vector<std::string> generated_declarations;
    std::vector<std::string> editor_discovery;
    std::vector<std::string> release_discovery;
    bool as_reference_debugging = false;
    bool resolve_object_ptr_callback_registered = false;
    bool has_invoke_return = false;
    std::string invoke_type;
    std::string invoke_kind;
    std::string invoke_value_json;
};

struct compiler_diagnostic {
    std::string severity;
    std::string code;
    std::string message;
    std::string source;
    std::uint32_t line = 0U;
    std::uint32_t column = 0U;
};

bool full_graph_request(const compile_request& request) noexcept {
    return request.request_version == wire::kRequestProtocolVersionV2 ||
        request.request_version == wire::kQualificationProtocolVersionV3;
}

std::string json_escape(const std::string_view text) {
    constexpr char hex[] = "0123456789abcdef";
    std::string output;
    output.reserve(text.size());
    for (const unsigned char ch : text) {
        switch (ch) {
        case '"': output += "\\\""; break;
        case '\\': output += "\\\\"; break;
        case '\b': output += "\\b"; break;
        case '\f': output += "\\f"; break;
        case '\n': output += "\\n"; break;
        case '\r': output += "\\r"; break;
        case '\t': output += "\\t"; break;
        default:
            if (ch < 0x20U) {
                output += "\\u00";
                output.push_back(hex[(ch >> 4U) & 0x0fU]);
                output.push_back(hex[ch & 0x0fU]);
            } else output.push_back(static_cast<char>(ch));
        }
    }
    return output;
}

sidecar_compile_result failure(
    const wire::ExitCode exit_code,
    const std::string_view kind,
    const std::string_view code,
    const std::string_view message,
    const std::vector<compiler_diagnostic>& diagnostics = {}) {
    std::string json = "{\"response_version\":1,\"ok\":false,\"failure_kind\":\"" +
        std::string(kind) + "\",\"diagnostics\":[";
    const auto append = [&](const compiler_diagnostic& diagnostic, const bool comma, std::string& target) {
        if (comma) target.push_back(',');
        target += "{\"severity\":\"" + diagnostic.severity + "\",\"code\":\"" +
            json_escape(diagnostic.code) + "\",\"message\":\"" +
            json_escape(diagnostic.message.substr(0U, wire::kMaxDiagnosticMessageBytes)) + "\"";
        if (!diagnostic.source.empty() && diagnostic.source.size() <= 16U * 1024U) {
            target += ",\"source_path\":\"" + json_escape(diagnostic.source) + "\"";
        }
        if (diagnostic.line != 0U) target += ",\"line\":" + std::to_string(diagnostic.line);
        if (diagnostic.column != 0U) target += ",\"column\":" + std::to_string(diagnostic.column);
        target.push_back('}');
    };
    if (diagnostics.empty()) {
        append({"error", std::string(code), std::string(message), {}, 0U, 0U}, false, json);
    } else {
        const std::size_t count = std::min(diagnostics.size(), wire::kMaxDiagnostics);
        for (std::size_t index = 0U; index < count; ++index) append(diagnostics[index], index != 0U, json);
    }
    json += "]}\n";
    if (json.size() > wire::kMaxResponseBytes) {
        json = "{\"response_version\":1,\"ok\":false,\"failure_kind\":\"internal\","
            "\"diagnostics\":[{\"severity\":\"error\",\"code\":\"GORE_AS_RESPONSE_LIMIT\","
            "\"message\":\"diagnostics exceeded the response limit\"}]}\n";
    }
    return {exit_code, std::move(json)};
}

sidecar_compile_result success(
    const compile_request& request,
    const std::uint64_t byte_len,
    const sha256_digest& digest,
    const std::vector<compiler_diagnostic>& diagnostics,
    const qualification_trace* const trace = nullptr) {
    std::string json = "{\"response_version\":1,\"ok\":true,\"output\":{\"cache_path\":\"" +
        json_escape(request.output_path_utf8) + "\",\"byte_len\":" + std::to_string(byte_len) +
        ",\"sha256\":\"" + sha256_hex(digest) + "\",\"profile_sha256\":\"" +
        sha256_hex(request.profile_sha256) + "\"},\"diagnostics\":[";
    const std::size_t diagnostic_count = std::min(diagnostics.size(), wire::kMaxDiagnostics);
    for (std::size_t index = 0U; index < diagnostic_count; ++index) {
        if (index != 0U) json.push_back(',');
        const auto& diagnostic = diagnostics[index];
        json += "{\"severity\":\"" + diagnostic.severity + "\",\"code\":\"" +
            json_escape(diagnostic.code) + "\",\"message\":\"" +
            json_escape(diagnostic.message.substr(0U, wire::kMaxDiagnosticMessageBytes)) + "\"";
        if (!diagnostic.source.empty() && diagnostic.source.size() <= 16U * 1024U) {
            json += ",\"source_path\":\"" + json_escape(diagnostic.source) + "\"";
        }
        if (diagnostic.line != 0U) json += ",\"line\":" + std::to_string(diagnostic.line);
        if (diagnostic.column != 0U) json += ",\"column\":" + std::to_string(diagnostic.column);
        json.push_back('}');
    }
    json += "]";
    if (request.qualification_mode) {
        if (trace == nullptr) {
            return failure(wire::ExitCode::software, "internal", "GORE_AS_QUALIFICATION_TRACE_MISSING",
                "qualification success omitted its same-run trace");
        }
        const auto append_strings = [&](const std::vector<std::string>& values, std::string& target) {
            target.push_back('[');
            for (std::size_t index = 0U; index < values.size(); ++index) {
                if (index != 0U) target.push_back(',');
                target += "\"" + json_escape(values[index]) + "\"";
            }
            target.push_back(']');
        };
        const auto append_captures = [&](const std::vector<qualification_hook_capture>& values, std::string& target) {
            target.push_back('[');
            for (std::size_t index = 0U; index < values.size(); ++index) {
                if (index != 0U) target.push_back(',');
                target += "{\"subject_identity\":\"" + json_escape(values[index].subject_identity) +
                    "\",\"generated_declarations\":";
                append_strings(values[index].generated_declarations, target);
                target.push_back('}');
            }
            target.push_back(']');
        };
        json += ",\"qualification\":{\"protocol_version\":3,\"suite_id\":\"" +
            json_escape(request.qualification.suite_id) + "\",\"corpus_sha256\":\"" +
            sha256_hex(request.qualification.corpus_sha256) + "\",\"case_id\":\"" +
            json_escape(request.qualification.case_id) + "\",\"phase\":\"" +
            json_escape(request.qualification.phase) + "\",\"request_sha256\":\"" +
            sha256_hex(request.qualification.request_sha256) + "\",\"build_flags\":{"
            "\"as_reference_debugging\":" +
            std::string(trace->as_reference_debugging ? "true" : "false") +
            ",\"resolve_object_ptr_callback_registered\":" +
            std::string(trace->resolve_object_ptr_callback_registered ? "true" : "false") +
            "},\"frontend\":{\"class_analyze_bound\":" +
            std::string(trace->class_analyze_bound ? "true" : "false") +
            ",\"class_analyze_captures\":";
        append_captures(trace->class_analyze, json);
        json += ",\"process_chunks_bound\":" +
            std::string(trace->process_chunks_bound ? "true" : "false") +
            ",\"process_chunks_captures\":";
        append_captures(trace->process_chunks, json);
        json += ",\"post_process_code_bound\":" +
            std::string(trace->post_process_code_bound ? "true" : "false") +
            ",\"post_process_captures\":";
        append_captures(trace->post_process, json);
        json += ",\"generated_declarations\":";
        append_strings(trace->generated_declarations, json);
        json += ",\"editor_discovery\":";
        append_strings(trace->editor_discovery, json);
        json += ",\"release_discovery\":";
        append_strings(trace->release_discovery, json);
        json += "},\"invoke_return\":";
        if (trace->has_invoke_return) {
            json += "{\"type_identity\":\"" + json_escape(trace->invoke_type) +
                "\",\"value\":{\"kind\":\"" + json_escape(trace->invoke_kind) +
                "\",\"value\":" + trace->invoke_value_json + "}}";
        } else {
            json += "null";
        }
        json.push_back('}');
    }
    json += "}\n";
    if (json.size() > wire::kMaxResponseBytes) {
        return failure(wire::ExitCode::software, "internal", "GORE_AS_RESPONSE_LIMIT", "success response exceeded the protocol limit");
    }
    return {wire::ExitCode::success, std::move(json)};
}

bool utf8_to_wide(const std::string& input, std::wstring& output, std::string& detail) {
    if (input.empty() || input.find('\0') != std::string::npos) {
        detail = "path is empty or contains NUL";
        return false;
    }
    const int count = MultiByteToWideChar(
        CP_UTF8, MB_ERR_INVALID_CHARS, input.data(), static_cast<int>(input.size()), nullptr, 0);
    if (count <= 0 || static_cast<std::size_t>(count) > wire::kMaxRequestPathUtf16Units) {
        detail = "path is not bounded canonical UTF-8";
        return false;
    }
    std::wstring staged(static_cast<std::size_t>(count), L'\0');
    if (MultiByteToWideChar(
            CP_UTF8, MB_ERR_INVALID_CHARS, input.data(), static_cast<int>(input.size()),
            staged.data(), count) != count) {
        detail = "path UTF-8 conversion failed";
        return false;
    }
    output = std::move(staged);
    return true;
}

bool absolute_path(const std::string& input, std::wstring& output, std::string& detail) {
    std::wstring wide;
    if (!utf8_to_wide(input, wide, detail)) return false;
    if (wide.size() < 3U || !((wide[1] == L':' && (wide[2] == L'\\' || wide[2] == L'/')) ||
        (wide[0] == L'\\' && wide[1] == L'\\'))) {
        detail = "protocol path is not absolute";
        return false;
    }
    const DWORD required = GetFullPathNameW(wide.c_str(), 0U, nullptr, nullptr);
    if (required == 0U || required > wire::kMaxRequestPathUtf16Units) {
        detail = "protocol path could not be normalized";
        return false;
    }
    std::wstring normalized(required, L'\0');
    const DWORD written = GetFullPathNameW(wide.c_str(), required, normalized.data(), nullptr);
    if (written == 0U || written >= required) {
        detail = "protocol path normalization failed";
        return false;
    }
    normalized.resize(written);
    std::replace(wide.begin(), wide.end(), L'/', L'\\');
    if (_wcsicmp(wide.c_str(), normalized.c_str()) != 0) {
        detail = "protocol path is not lexically normalized";
        return false;
    }
    output = std::move(normalized);
    return true;
}

bool inspect_regular(const HANDLE file, std::uint64_t& size, std::string& detail) {
    if (GetFileType(file) != FILE_TYPE_DISK) { detail = "path is not a regular disk file"; return false; }
    FILE_ATTRIBUTE_TAG_INFO attributes{};
    if (!GetFileInformationByHandleEx(file, FileAttributeTagInfo, &attributes, sizeof(attributes))) {
        detail = "file metadata query failed";
        return false;
    }
    if ((attributes.FileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY)) != 0U) {
        detail = "file is a directory or reparse point";
        return false;
    }
    LARGE_INTEGER measured{};
    if (!GetFileSizeEx(file, &measured) || measured.QuadPart < 0) {
        detail = "file size query failed";
        return false;
    }
    size = static_cast<std::uint64_t>(measured.QuadPart);
    return true;
}

bool inspect_directory(const std::wstring& path, std::string& detail) {
    unique_handle directory(CreateFileW(
        path.c_str(), FILE_READ_ATTRIBUTES, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
    if (!directory.valid() || GetFileType(directory.get()) != FILE_TYPE_DISK) {
        detail = "directory could not be opened for sealed inspection";
        return false;
    }
    FILE_ATTRIBUTE_TAG_INFO attributes{};
    if (!GetFileInformationByHandleEx(
            directory.get(), FileAttributeTagInfo, &attributes, sizeof(attributes)) ||
        (attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0U ||
        (attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0U) {
        detail = "directory is missing or is a reparse point";
        return false;
    }
    return true;
}

bool path_below(const std::wstring& root, const std::wstring& path) {
    if (path.size() <= root.size() || _wcsnicmp(root.c_str(), path.c_str(), root.size()) != 0) return false;
    return path[root.size()] == L'\\';
}

bool read_file(
    const std::wstring& path,
    const std::uint64_t maximum,
    std::vector<std::uint8_t>& output,
    std::string& detail,
    const std::uint64_t* expected_size = nullptr,
    const sha256_digest* expected_hash = nullptr) {
    unique_handle file(CreateFileW(
        path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
    if (!file.valid()) { detail = "file could not be opened for sealed read"; return false; }
    std::uint64_t size = 0U;
    if (!inspect_regular(file.get(), size, detail)) return false;
    if (size > maximum || size > std::numeric_limits<std::size_t>::max()) {
        detail = "file exceeds its protocol byte limit";
        return false;
    }
    if (expected_size != nullptr && size != *expected_size) {
        detail = "file length does not match its seal";
        return false;
    }
    std::vector<std::uint8_t> staged(static_cast<std::size_t>(size));
    sha256 hash;
    std::size_t offset = 0U;
    while (offset < staged.size()) {
        const DWORD wanted = static_cast<DWORD>(std::min<std::size_t>(64U * 1024U, staged.size() - offset));
        DWORD received = 0U;
        if (!ReadFile(file.get(), staged.data() + offset, wanted, &received, nullptr) || received == 0U) {
            detail = "sealed file read was truncated";
            return false;
        }
        hash.update(staged.data() + offset, received);
        offset += received;
    }
    std::uint8_t sentinel = 0U;
    DWORD extra = 0U;
    if (!ReadFile(file.get(), &sentinel, 1U, &extra, nullptr) || extra != 0U) {
        detail = "sealed file changed while being read";
        return false;
    }
    if (expected_hash != nullptr && hash.finish() != *expected_hash) {
        detail = "file SHA-256 does not match its seal";
        return false;
    }
    output = std::move(staged);
    return true;
}

bool read_text(
    const std::wstring& path,
    const std::uint64_t maximum,
    std::string& output,
    std::string& detail,
    const std::uint64_t* expected_size = nullptr,
    const sha256_digest* expected_hash = nullptr) {
    std::vector<std::uint8_t> bytes;
    if (!read_file(path, maximum, bytes, detail, expected_size, expected_hash)) return false;
    if (std::find(bytes.begin(), bytes.end(), 0U) != bytes.end()) {
        detail = "text file contains NUL";
        return false;
    }
    // JSON payloads receive full UTF-8 validation in json::parse. Source text
    // is checked below through MultiByteToWideChar without altering bytes.
    output.assign(reinterpret_cast<const char*>(bytes.data()), bytes.size());
    return true;
}

bool safe_relative_path(const std::string& path) {
    if (path.empty() || path.size() > wire::kMaxModuleIdentityBytes || path.front() == '/' ||
        path.find('\\') != std::string::npos || path.find(':') != std::string::npos ||
        path.find('\0') != std::string::npos) return false;
    std::size_t begin = 0U;
    while (begin <= path.size()) {
        const std::size_t end = path.find('/', begin);
        const std::string_view part(path.data() + begin, (end == std::string::npos ? path.size() : end) - begin);
        if (part.empty() || part == "." || part == "..") return false;
        if (end == std::string::npos) break;
        begin = end + 1U;
    }
    return true;
}

bool valid_module_name(const std::string& name, std::string& detail) {
    if (name.empty() || name.size() > wire::kMaxModuleIdentityBytes ||
        name.find('\0') != std::string::npos) {
        detail = "module name is empty, oversized, or contains NUL";
        return false;
    }
    std::wstring wide;
    if (!utf8_to_wide(name, wide, detail)) return false;
    if (std::any_of(wide.begin(), wide.end(), [](const wchar_t ch) {
            return std::iswcntrl(static_cast<wint_t>(ch)) != 0;
        })) {
        detail = "module name contains a control character";
        return false;
    }
    return true;
}

bool lower_utf8(const std::string& input, std::string& output, std::string& detail) {
    std::wstring wide;
    if (!utf8_to_wide(input, wide, detail)) return false;
    const int required = LCMapStringEx(
        LOCALE_NAME_INVARIANT, LCMAP_LOWERCASE, wide.data(),
        static_cast<int>(wide.size()), nullptr, 0, nullptr, nullptr, 0U);
    if (required <= 0) {
        detail = "identity case folding failed";
        return false;
    }
    std::wstring lowered(static_cast<std::size_t>(required), L'\0');
    if (LCMapStringEx(
            LOCALE_NAME_INVARIANT, LCMAP_LOWERCASE, wide.data(),
            static_cast<int>(wide.size()), lowered.data(), required,
            nullptr, nullptr, 0U) != required) {
        detail = "identity case folding changed while mapping";
        return false;
    }
    const int utf8_size = WideCharToMultiByte(
        CP_UTF8, WC_ERR_INVALID_CHARS, lowered.data(), required,
        nullptr, 0, nullptr, nullptr);
    if (utf8_size <= 0) {
        detail = "folded identity is not canonical UTF-8";
        return false;
    }
    std::string staged(static_cast<std::size_t>(utf8_size), '\0');
    if (WideCharToMultiByte(
            CP_UTF8, WC_ERR_INVALID_CHARS, lowered.data(), required,
            staged.data(), utf8_size, nullptr, nullptr) != utf8_size) {
        detail = "folded identity UTF-8 conversion failed";
        return false;
    }
    output = std::move(staged);
    return true;
}

struct identity_sort_key {
    std::string folded_module;
    std::string folded_path;
    std::string module_name;
    std::string relative_path;
};

bool identity_key(
    const std::string& module_name,
    const std::string& relative_path,
    identity_sort_key& output,
    std::string& detail) {
    if (!valid_module_name(module_name, detail) || !safe_relative_path(relative_path) ||
        !lower_utf8(module_name, output.folded_module, detail) ||
        !lower_utf8(relative_path, output.folded_path, detail)) {
        if (detail.empty()) detail = "module identity is invalid";
        return false;
    }
    output.module_name = module_name;
    output.relative_path = relative_path;
    return true;
}

bool identity_less(const identity_sort_key& left, const identity_sort_key& right) noexcept {
    if (left.folded_module != right.folded_module) {
        return left.folded_module < right.folded_module;
    }
    if (left.folded_path != right.folded_path) {
        return left.folded_path < right.folded_path;
    }
    if (left.module_name != right.module_name) return left.module_name < right.module_name;
    return left.relative_path < right.relative_path;
}

std::wstring joined(const std::wstring& root, const std::string& relative, std::string& detail) {
    std::wstring wide;
    if (!utf8_to_wide(relative, wide, detail)) return {};
    std::replace(wide.begin(), wide.end(), L'/', L'\\');
    return root + L"\\" + wide;
}

bool parse_path_seal(const json::value& input, path_seal& output, std::string& detail) {
    std::string digest;
    if (!json::require_object_keys(input, {"path", "byte_len", "sha256"}, {}, detail) ||
        !json::get_string(input, "path", output.path_utf8, detail) ||
        !absolute_path(output.path_utf8, output.path, detail) ||
        !json::get_u64(input, "byte_len", output.byte_len, detail) || output.byte_len == 0U ||
        !json::get_string(input, "sha256", digest, detail) ||
        !parse_sha256_hex(digest, output.sha256)) {
        if (detail.empty()) detail = "sealed path has an invalid SHA-256";
        return false;
    }
    return true;
}

bool parse_request(
    const std::string_view bytes,
    compile_request& output,
    std::string& detail,
    const bool allow_qualification) {
    json::value root;
    json::parse_error error;
    if (!json::parse(bytes, wire::kMaxJsonNestingDepth, root, error)) {
        detail = "request JSON offset " + std::to_string(error.offset) + ": " + error.detail;
        return false;
    }
    const json::value* qualification = nullptr;
    std::uint64_t version = 0U;
    std::string operation;
    if (!json::get_u64(root, "request_version", version, detail) ||
        !json::get_string(root, "operation", operation, detail)) {
        detail = "unsupported request version or operation";
        return false;
    }
    const bool qualification_mode = version == wire::kQualificationProtocolVersionV3 &&
        operation == "qualify" && allow_qualification;
    if (!(version == wire::kRequestProtocolVersionV1 ||
          version == wire::kRequestProtocolVersionV2 || qualification_mode) ||
        ((version == wire::kRequestProtocolVersionV1 || version == wire::kRequestProtocolVersionV2) &&
         operation != "compile") ||
        !json::require_object_keys(root,
            {"request_version", "operation", "profile", "inputs", "output"},
            qualification_mode ? std::initializer_list<std::string_view>{"qualification"}
                               : std::initializer_list<std::string_view>{}, detail) ||
        (qualification_mode && !json::get_object(root, "qualification", qualification, detail))) {
        detail = "unsupported request version or operation";
        return false;
    }
    compile_request staged;
    staged.request_version = static_cast<std::uint32_t>(version);
    staged.qualification_mode = qualification_mode;
    if (qualification_mode) {
        std::string corpus_digest;
        if (!json::require_object_keys(*qualification,
                {"suite_id", "corpus_sha256", "case_id", "phase", "invoke_module", "invoke_declaration"},
                {}, detail) ||
            !json::get_string(*qualification, "suite_id", staged.qualification.suite_id, detail) ||
            !json::get_string(*qualification, "corpus_sha256", corpus_digest, detail) ||
            !parse_sha256_hex(corpus_digest, staged.qualification.corpus_sha256) ||
            !json::get_string(*qualification, "case_id", staged.qualification.case_id, detail) ||
            !json::get_string(*qualification, "phase", staged.qualification.phase, detail) ||
            !json::get_string(*qualification, "invoke_module", staged.qualification.invoke_module, detail) ||
            !json::get_string(*qualification, "invoke_declaration", staged.qualification.invoke_declaration, detail) ||
            staged.qualification.suite_id.empty() || staged.qualification.suite_id.size() > 1024U ||
            staged.qualification.case_id.empty() || staged.qualification.case_id.size() > 1024U ||
            (staged.qualification.phase != "single" &&
             staged.qualification.phase != "graph_baseline" &&
             staged.qualification.phase != "graph_final") ||
            staged.qualification.invoke_module.size() > wire::kMaxModuleIdentityBytes ||
            staged.qualification.invoke_declaration.size() > 4096U ||
            staged.qualification.invoke_module.empty() != staged.qualification.invoke_declaration.empty()) {
            if (detail.empty()) detail = "qualification identity/mode is invalid";
            return false;
        }
        staged.qualification.request_sha256 = sha256_bytes(bytes.data(), bytes.size());
    }
    const json::value* profile = nullptr;
    std::string digest;
    if (!json::get_object(root, "profile", profile, detail) ||
        !json::require_object_keys(*profile,
            {"manifest_path", "profile_root", "profile_sha256", "steam_build_id", "depot_id",
             "depot_manifest_gid", "required_probe_suite_version"}, {}, detail) ||
        !json::get_string(*profile, "manifest_path", staged.manifest_path_utf8, detail) ||
        !absolute_path(staged.manifest_path_utf8, staged.manifest_path, detail) ||
        !json::get_string(*profile, "profile_root", staged.profile_root_utf8, detail) ||
        !absolute_path(staged.profile_root_utf8, staged.profile_root, detail) ||
        !json::get_string(*profile, "profile_sha256", digest, detail) ||
        !parse_sha256_hex(digest, staged.profile_sha256) ||
        !json::get_u64(*profile, "steam_build_id", staged.steam_build_id, detail)) return false;
    std::uint64_t depot = 0U;
    if (!json::get_u64(*profile, "depot_id", depot, detail) || depot > std::numeric_limits<std::uint32_t>::max() ||
        !json::get_u64(*profile, "depot_manifest_gid", staged.depot_manifest_gid, detail) ||
        !json::get_string(*profile, "required_probe_suite_version", staged.required_probe_suite_version, detail)) return false;
    staged.depot_id = static_cast<std::uint32_t>(depot);

    const json::value* inputs = nullptr;
    const json::value* base = nullptr;
    const json::value* binds = nullptr;
    const json::value* source_tree = nullptr;
    const json::value* files = nullptr;
    const json::value* operations = nullptr;
    const json::value* final_manifest = nullptr;
    if (!json::get_object(root, "inputs", inputs, detail) ||
        !(staged.request_version == wire::kRequestProtocolVersionV1
            ? json::require_object_keys(
                *inputs, {"base_cache", "binds_cache", "source_tree", "overlays"}, {}, detail)
            : json::require_object_keys(
                *inputs, {"base_cache", "binds_cache", "source_tree", "changes", "final_manifest"}, {}, detail)) ||
        !json::get_object(*inputs, "base_cache", base, detail) || !parse_path_seal(*base, staged.base_cache, detail) ||
        !json::get_object(*inputs, "binds_cache", binds, detail) || !parse_path_seal(*binds, staged.binds_cache, detail) ||
        !json::get_object(*inputs, "source_tree", source_tree, detail) ||
        !json::require_object_keys(*source_tree, {"root", "files"}, {}, detail) ||
        !json::get_string(*source_tree, "root", staged.source_root_utf8, detail) ||
        !absolute_path(staged.source_root_utf8, staged.source_root, detail) ||
        !json::get_array(*source_tree, "files", files, detail) || files->elements.size() > wire::kMaxSourceFiles) {
        return false;
    }
    if (staged.request_version == wire::kRequestProtocolVersionV1) {
        if (!json::get_array(*inputs, "overlays", operations, detail) ||
            operations->elements.empty() || operations->elements.size() > wire::kMaxOverlayModules) {
            return false;
        }
    } else if (!json::get_array(*inputs, "changes", operations, detail) ||
        operations->elements.size() > wire::kMaxOverlayModules ||
        !json::get_array(*inputs, "final_manifest", final_manifest, detail) ||
        final_manifest->elements.empty() || final_manifest->elements.size() > wire::kMaxSourceFiles) {
        return false;
    }

    std::set<std::string> file_paths;
    std::string previous_file_path;
    std::uint64_t aggregate = 0U;
    for (const auto& item : files->elements) {
        source_file file;
        std::string file_digest;
        if (!json::require_object_keys(item, {"path", "byte_len", "sha256"}, {}, detail) ||
            !json::get_string(item, "path", file.relative_path, detail) || !safe_relative_path(file.relative_path) ||
            !json::get_u64(item, "byte_len", file.byte_len, detail) || file.byte_len > wire::kMaxSourceFileBytes ||
            !json::get_string(item, "sha256", file_digest, detail) || !parse_sha256_hex(file_digest, file.sha256) ||
            (!previous_file_path.empty() && file.relative_path <= previous_file_path) ||
            !file_paths.insert(file.relative_path).second) {
            if (detail.empty()) detail = "source tree contains an invalid or duplicate file";
            return false;
        }
        if (aggregate > wire::kMaxAggregateSourceBytes - file.byte_len) {
            detail = "source tree exceeds the aggregate byte limit";
            return false;
        }
        aggregate += file.byte_len;
        previous_file_path = file.relative_path;
        staged.source_files.push_back(std::move(file));
    }

    if (staged.request_version == wire::kRequestProtocolVersionV1) {
        std::set<std::string> module_names;
        std::set<std::string> overlay_paths;
        for (const auto& item : operations->elements) {
            overlay_module overlay;
            std::string operation_name;
            std::uint64_t ordinal = 0U;
            if (!json::require_object_keys(item, {"ordinal", "operation", "module_name", "relative_path"}, {}, detail) ||
                !json::get_u64(item, "ordinal", ordinal, detail) || ordinal != staged.overlays.size() ||
                !json::get_string(item, "operation", operation_name, detail) ||
                !json::get_string(item, "module_name", overlay.module_name, detail) || overlay.module_name.empty() ||
                overlay.module_name.size() > wire::kMaxModuleIdentityBytes ||
                !json::get_string(item, "relative_path", overlay.relative_path, detail) ||
                !safe_relative_path(overlay.relative_path) || file_paths.count(overlay.relative_path) != 1U ||
                !module_names.insert(overlay.module_name).second || !overlay_paths.insert(overlay.relative_path).second) {
                if (detail.empty()) detail = "overlay manifest is invalid or colliding";
                return false;
            }
            overlay.ordinal = static_cast<std::uint32_t>(ordinal);
            if (operation_name == "add") overlay.operation = preprocessor_source::operation::add;
            else if (operation_name == "edit") overlay.operation = preprocessor_source::operation::edit;
            else { detail = "unsupported overlay operation"; return false; }
            staged.overlays.push_back(std::move(overlay));
        }
    } else {
        std::map<std::string, const source_file*> source_by_path;
        for (const source_file& file : staged.source_files) {
            source_by_path.emplace(file.relative_path, &file);
        }
        std::set<std::string> used_sources;
        std::set<std::string> module_names;
        std::set<std::string> change_paths;
        identity_sort_key previous;
        bool has_previous = false;
        for (const auto& item : operations->elements) {
            graph_change change;
            std::string operation_name;
            std::string source_digest;
            bool has_length = false;
            bool has_digest = false;
            std::uint64_t ordinal = 0U;
            if (!json::require_object_keys(
                    item, {"ordinal", "operation", "module_name", "relative_path"},
                    {"source_byte_len", "source_sha256"}, detail) ||
                !json::get_u64(item, "ordinal", ordinal, detail) || ordinal != staged.changes.size() ||
                !json::get_string(item, "operation", operation_name, detail) ||
                !json::get_string(item, "module_name", change.module_name, detail) ||
                !json::get_string(item, "relative_path", change.relative_path, detail) ||
                !json::get_optional_u64(item, "source_byte_len", has_length, change.source_byte_len, detail) ||
                !json::get_optional_string(item, "source_sha256", has_digest, source_digest, detail) ||
                has_length != has_digest ||
                (has_digest && !parse_sha256_hex(source_digest, change.source_sha256))) {
                if (detail.empty()) detail = "FullGraph change is malformed";
                return false;
            }
            change.ordinal = static_cast<std::uint32_t>(ordinal);
            if (operation_name == "add") change.operation = graph_change_operation::add;
            else if (operation_name == "edit") change.operation = graph_change_operation::edit;
            else if (operation_name == "delete") change.operation = graph_change_operation::remove;
            else { detail = "unsupported FullGraph change operation"; return false; }
            change.has_source = has_length;

            identity_sort_key key;
            if (!identity_key(change.module_name, change.relative_path, key, detail) ||
                (has_previous && !identity_less(previous, key)) ||
                !module_names.insert(key.folded_module).second ||
                !change_paths.insert(key.folded_path).second) {
                if (detail.empty()) detail = "FullGraph changes are not canonical and collision-free";
                return false;
            }
            previous = std::move(key);
            has_previous = true;

            const auto source = source_by_path.find(change.relative_path);
            if (change.operation == graph_change_operation::remove) {
                if (change.has_source || source != source_by_path.end()) {
                    detail = "FullGraph Delete must not carry sealed source bytes";
                    return false;
                }
            } else if (!change.has_source || source == source_by_path.end() ||
                source->second->byte_len != change.source_byte_len ||
                source->second->sha256 != change.source_sha256 ||
                !used_sources.insert(change.relative_path).second) {
                detail = "FullGraph Add/Edit source is absent, duplicate, or disagrees with its seal";
                return false;
            }
            staged.changes.push_back(std::move(change));
        }
        if (used_sources.size() != staged.source_files.size()) {
            detail = "FullGraph source tree contains an undeclared source";
            return false;
        }

        std::set<std::string> final_names;
        std::set<std::string> final_paths;
        identity_sort_key previous_final;
        bool has_previous_final = false;
        for (const auto& item : final_manifest->elements) {
            final_module module;
            std::uint64_t ordinal = 0U;
            if (!json::require_object_keys(item, {"ordinal", "module_name", "relative_path"}, {}, detail) ||
                !json::get_u64(item, "ordinal", ordinal, detail) || ordinal != staged.final_manifest.size() ||
                !json::get_string(item, "module_name", module.module_name, detail) ||
                !json::get_string(item, "relative_path", module.relative_path, detail)) {
                return false;
            }
            identity_sort_key key;
            if (!identity_key(module.module_name, module.relative_path, key, detail) ||
                (has_previous_final && !identity_less(previous_final, key)) ||
                !final_names.insert(key.folded_module).second ||
                !final_paths.insert(key.folded_path).second) {
                if (detail.empty()) detail = "FullGraph final manifest is not canonical and collision-free";
                return false;
            }
            previous_final = std::move(key);
            has_previous_final = true;
            module.ordinal = static_cast<std::uint32_t>(ordinal);
            staged.final_manifest.push_back(std::move(module));
        }
    }

    const json::value* output_object = nullptr;
    if (!json::get_object(root, "output", output_object, detail) ||
        !json::require_object_keys(*output_object, {"cache_path"}, {}, detail) ||
        !json::get_string(*output_object, "cache_path", staged.output_path_utf8, detail) ||
        !absolute_path(staged.output_path_utf8, staged.output_path, detail)) return false;
    output = std::move(staged);
    return true;
}

bool validate_qualification_corpus_request(
    const std::string_view corpus_bytes,
    const compile_request& request,
    const std::map<std::string, std::string>& source_contents,
    std::string& detail) {
    if (!request.qualification_mode) return true;
    json::value root;
    json::parse_error error;
    if (!json::parse(corpus_bytes, wire::kMaxJsonNestingDepth, root, error) ||
        !json::require_object_keys(root,
            {"schema", "schema_version", "suite_id", "cases", "canonical_sha256"}, {}, detail)) {
        if (detail.empty()) detail = "sealed qualification corpus JSON is invalid";
        return false;
    }
    std::string schema, suite_id, canonical_digest;
    std::uint64_t schema_version = 0U;
    const json::value* cases = nullptr;
    if (!json::get_string(root, "schema", schema, detail) ||
        schema != "gore.as.compiler-probe-corpus" ||
        !json::get_u64(root, "schema_version", schema_version, detail) || schema_version != 2U ||
        !json::get_string(root, "suite_id", suite_id, detail) ||
        suite_id != request.qualification.suite_id ||
        !json::get_string(root, "canonical_sha256", canonical_digest, detail) ||
        canonical_digest != sha256_hex(request.qualification.corpus_sha256) ||
        !json::get_array(root, "cases", cases, detail)) {
        if (detail.empty()) detail = "qualification request does not match the sealed corpus identity";
        return false;
    }
    const json::value* selected_case = nullptr;
    for (const json::value& candidate : cases->elements) {
        std::string case_id;
        if (!json::get_string(candidate, "case_id", case_id, detail)) return false;
        if (case_id == request.qualification.case_id) {
            if (selected_case != nullptr) {
                detail = "sealed qualification corpus contains a duplicate case id";
                return false;
            }
            selected_case = &candidate;
        }
    }
    if (selected_case == nullptr || !json::require_object_keys(*selected_case,
            {"ordinal", "case_id", "category", "expected_outcome", "mode", "sections"}, {}, detail)) {
        if (detail.empty()) detail = "qualification case is absent from the sealed corpus";
        return false;
    }
    const json::value* mode = nullptr;
    const json::value* sections = nullptr;
    std::string mode_kind;
    if (!json::get_object(*selected_case, "mode", mode, detail) ||
        !json::get_string(*mode, "kind", mode_kind, detail)) return false;
    if (request.qualification.phase == "single") {
        if (mode_kind == "compile_only") {
            if (!json::require_object_keys(*mode, {"kind"}, {}, detail) ||
                !request.qualification.invoke_declaration.empty()) {
                detail = "compile-only corpus case cannot request invocation";
                return false;
            }
        } else if (mode_kind == "invoke") {
            std::string declaration;
            if (!json::require_object_keys(*mode, {"kind", "declaration"}, {}, detail) ||
                !json::get_string(*mode, "declaration", declaration, detail) ||
                declaration != request.qualification.invoke_declaration) {
                detail = "qualification invoke declaration is not the sealed corpus declaration";
                return false;
            }
        } else {
            detail = "graph-transition corpus case requires an explicit graph phase";
            return false;
        }
        if (!json::get_array(*selected_case, "sections", sections, detail)) return false;
    } else {
        if (mode_kind != "compile_graph_transition" ||
            !json::require_object_keys(*mode,
                {"kind", "baseline_sections", "changed_modules", "deleted_modules"}, {}, detail) ||
            !request.qualification.invoke_declaration.empty()) {
            detail = "qualification graph phase does not match the sealed corpus mode";
            return false;
        }
        if (request.qualification.phase == "graph_baseline") {
            if (!json::get_array(*mode, "baseline_sections", sections, detail)) return false;
        } else if (!json::get_array(*selected_case, "sections", sections, detail)) {
            return false;
        }
    }
    if (sections == nullptr || sections->elements.size() != request.source_files.size() ||
        source_contents.size() != request.source_files.size()) {
        detail = "qualification source count differs from the sealed corpus case/phase";
        return false;
    }
    std::map<std::string, std::string> requested_modules;
    for (const graph_change& change : request.changes) {
        if (change.operation != graph_change_operation::remove) {
            requested_modules.emplace(change.relative_path, change.module_name);
        }
    }
    for (std::size_t index = 0U; index < sections->elements.size(); ++index) {
        const json::value& section = sections->elements[index];
        std::uint64_t ordinal = 0U;
        std::string module, relative_path, source, source_digest;
        sha256_digest parsed_digest{};
        if (!json::require_object_keys(section,
                {"ordinal", "module", "relative_path", "source_utf8", "source_sha256"}, {}, detail) ||
            !json::get_u64(section, "ordinal", ordinal, detail) || ordinal != index ||
            !json::get_string(section, "module", module, detail) ||
            !json::get_string(section, "relative_path", relative_path, detail) ||
            !json::get_string(section, "source_utf8", source, detail) ||
            !json::get_string(section, "source_sha256", source_digest, detail) ||
            !parse_sha256_hex(source_digest, parsed_digest) ||
            parsed_digest != sha256_bytes(source.data(), source.size()) ||
            source_contents.find(relative_path) == source_contents.end() ||
            source_contents.at(relative_path) != source ||
            requested_modules.find(relative_path) == requested_modules.end() ||
            requested_modules.at(relative_path) != module) {
            if (detail.empty()) detail = "qualification source differs from its sealed corpus row";
            return false;
        }
    }
    if (!request.qualification.invoke_declaration.empty()) {
        if (sections->elements.size() != 1U) {
            detail = "safe qualification invocation requires one sealed source module";
            return false;
        }
        std::string module;
        if (!json::get_string(sections->elements[0], "module", module, detail) ||
            module != request.qualification.invoke_module) {
            detail = "qualification invoke module is not the sealed corpus source module";
            return false;
        }
    }
    return true;
}

bool is_valid_utf8(const std::string_view text) {
    if (text.size() > static_cast<std::size_t>(std::numeric_limits<int>::max())) return false;
    if (text.empty()) return true;
    const int wide = MultiByteToWideChar(
        CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), static_cast<int>(text.size()), nullptr, 0);
    return wide > 0;
}

bool write_output_new(const std::wstring& path, const std::vector<std::uint8_t>& bytes, std::string& detail) {
    const std::wstring temporary = path + L".gore-tmp-" + std::to_wstring(GetCurrentProcessId()) +
        L"-" + std::to_wstring(GetTickCount64());
    if (temporary.size() > wire::kMaxRequestPathUtf16Units) {
        detail = "output temporary path exceeds the protocol limit";
        return false;
    }
    bool written_ok = false;
    {
        unique_handle output(CreateFileW(
            temporary.c_str(), GENERIC_WRITE, 0U, nullptr, CREATE_NEW,
            FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_WRITE_THROUGH, nullptr));
        if (!output.valid()) { detail = "output staging path could not be created"; return false; }
        std::size_t offset = 0U;
        while (offset < bytes.size()) {
            const DWORD wanted = static_cast<DWORD>(std::min<std::size_t>(64U * 1024U, bytes.size() - offset));
            DWORD written = 0U;
            if (!WriteFile(output.get(), bytes.data() + offset, wanted, &written, nullptr) || written != wanted) {
                detail = "output cache staging write failed";
                break;
            }
            offset += written;
        }
        if (offset == bytes.size() && FlushFileBuffers(output.get())) written_ok = true;
        else if (detail.empty()) detail = "output cache staging flush failed";
    }
    if (!written_ok) {
        (void)DeleteFileW(temporary.c_str());
        return false;
    }
    if (!MoveFileExW(temporary.c_str(), path.c_str(), MOVEFILE_WRITE_THROUGH)) {
        (void)DeleteFileW(temporary.c_str());
        detail = "output path could not be published with create-new semantics";
        return false;
    }
    return true;
}

std::vector<preprocessor_base_module> base_modules(const cache_wire::cache& cache) {
    std::vector<preprocessor_base_module> output;
    output.reserve(cache.modules.size());
    for (const auto& pair : cache.modules) {
        preprocessor_base_module module;
        module.module_name = pair.second.module_name.bytes;
        module.classes.reserve(pair.second.classes.size());
        for (const auto& input : pair.second.classes) {
            preprocessor_base_class type;
            type.class_name = input.class_name.bytes;
            type.name_space = input.name_space.bytes;
            type.super_class = input.super_class.bytes;
            type.code_super_class = input.code_super_class.bytes;
            type.super_is_code_class = input.super_is_code_class;
            type.is_struct = (static_cast<std::uint32_t>(input.flags) & asOBJ_VALUE) != 0U;
            module.classes.push_back(std::move(type));
        }
        output.push_back(std::move(module));
    }
    return output;
}

bool validate_and_apply_full_graph(
    const compile_request& request,
    cache_wire::cache& base,
    std::string& detail) {
    if (!full_graph_request(request)) return true;
    if (base.modules.size() > wire::kMaxSourceFiles) {
        detail = "base module graph exceeds the FullGraph protocol bound";
        return false;
    }

    struct base_identity {
        std::size_t index = 0U;
        std::string module_name;
        std::string relative_path;
    };
    std::map<std::string, base_identity> base_by_name;
    std::set<std::string> base_paths;
    for (std::size_t index = 0U; index < base.modules.size(); ++index) {
        const auto& module = base.modules[index].second;
        identity_sort_key key;
        if (!identity_key(
                module.module_name.bytes, module.script_relative_filename.bytes,
                key, detail) ||
            !base_by_name.emplace(
                key.folded_module,
                base_identity{index, module.module_name.bytes,
                    module.script_relative_filename.bytes}).second ||
            !base_paths.insert(key.folded_path).second) {
            if (detail.empty()) detail = "base cache has invalid or case-colliding module identities";
            return false;
        }
    }

    std::map<std::string, std::pair<std::string, std::string>> expected;
    for (const auto& item : base_by_name) {
        expected.emplace(
            item.first,
            std::make_pair(item.second.module_name, item.second.relative_path));
    }
    std::set<std::string> deleted_names;
    for (const graph_change& change : request.changes) {
        identity_sort_key key;
        if (!identity_key(change.module_name, change.relative_path, key, detail)) return false;
        const auto existing = base_by_name.find(key.folded_module);
        if (change.operation == graph_change_operation::add) {
            if (existing != base_by_name.end() || base_paths.count(key.folded_path) != 0U ||
                !expected.emplace(
                    key.folded_module,
                    std::make_pair(change.module_name, change.relative_path)).second) {
                detail = "FullGraph Add collides with a sealed base module identity";
                return false;
            }
            continue;
        }
        if (existing == base_by_name.end() ||
            existing->second.module_name != change.module_name ||
            existing->second.relative_path != change.relative_path) {
            detail = change.operation == graph_change_operation::edit
                ? "FullGraph Edit does not exactly identify a sealed base module"
                : "FullGraph Delete does not exactly identify a sealed base module";
            return false;
        }
        if (change.operation == graph_change_operation::remove) {
            if (expected.erase(key.folded_module) != 1U ||
                !deleted_names.insert(change.module_name).second) {
                detail = "FullGraph Delete was duplicated or could not remove its base identity";
                return false;
            }
        }
    }

    if (expected.size() != request.final_manifest.size()) {
        detail = "FullGraph final manifest count differs from the declared transformation";
        return false;
    }
    for (const final_module& module : request.final_manifest) {
        identity_sort_key key;
        if (!identity_key(module.module_name, module.relative_path, key, detail)) return false;
        const auto found = expected.find(key.folded_module);
        if (found == expected.end() || found->second.first != module.module_name ||
            found->second.second != module.relative_path) {
            detail = "FullGraph final manifest differs from base plus Add/Edit/Delete";
            return false;
        }
    }

    base.modules.erase(
        std::remove_if(
            base.modules.begin(), base.modules.end(),
            [&](const auto& entry) {
                return deleted_names.count(entry.second.module_name.bytes) != 0U;
            }),
        base.modules.end());
    if (base.modules.size() +
            std::count_if(
                request.changes.begin(), request.changes.end(),
                [](const graph_change& change) {
                    return change.operation == graph_change_operation::add;
                }) != request.final_manifest.size()) {
        detail = "FullGraph base pruning did not produce the declared final graph size";
        return false;
    }
    return true;
}

bool validate_full_graph_output(
    const compile_request& request,
    const cache_wire::cache& generated,
    std::string& detail) {
    if (!full_graph_request(request)) return true;
    if (generated.modules.size() != request.final_manifest.size()) {
        detail = "compiled cache module count differs from the FullGraph final manifest";
        return false;
    }
    std::map<std::string, std::pair<std::string, std::string>> actual;
    std::set<std::string> actual_paths;
    for (const auto& entry : generated.modules) {
        const auto& module = entry.second;
        identity_sort_key key;
        if (!identity_key(
                module.module_name.bytes, module.script_relative_filename.bytes,
                key, detail) ||
            !actual.emplace(
                key.folded_module,
                std::make_pair(module.module_name.bytes,
                    module.script_relative_filename.bytes)).second ||
            !actual_paths.insert(key.folded_path).second) {
            if (detail.empty()) detail = "compiled cache has invalid or colliding module identities";
            return false;
        }
    }
    for (const final_module& module : request.final_manifest) {
        identity_sort_key key;
        if (!identity_key(module.module_name, module.relative_path, key, detail)) return false;
        const auto found = actual.find(key.folded_module);
        if (found == actual.end() || found->second.first != module.module_name ||
            found->second.second != module.relative_path) {
            detail = "compiled cache manifest does not exactly match the FullGraph final manifest";
            return false;
        }
    }
    return true;
}

void hash_u64(sha256& hash, const std::uint64_t value) noexcept {
    std::array<std::uint8_t, 8U> bytes{};
    for (std::size_t index = 0U; index < bytes.size(); ++index) {
        bytes[index] = static_cast<std::uint8_t>(value >> (index * 8U));
    }
    hash.update(bytes.data(), bytes.size());
}

void hash_field(sha256& hash, const std::string_view value) noexcept {
    hash_u64(hash, value.size());
    hash.update(value);
}

sha256_digest graph_input_digest(
    preprocessor_graph_hook_module* const modules,
    const std::size_t module_count) noexcept {
    sha256 hash;
    constexpr char domain[] = "gore-as-external-hook-graph-input-v1\0";
    hash.update(domain, sizeof(domain) - 1U);
    hash_u64(hash, module_count);
    for (std::size_t index = 0U; index < module_count; ++index) {
        const lexical_module_description* const module = modules[index].module;
        if (module == nullptr) {
            hash_field(hash, {});
            hash_u64(hash, 0U);
            continue;
        }
        hash_field(hash, module->module_name);
        hash_u64(hash, module->code.size());
        for (const preprocessed_code_section& section : module->code) {
            hash_field(hash, section.relative_path);
            hash_field(hash, section.conditioned_code);
        }
    }
    return hash.finish();
}

sha256_digest graph_output_digest(
    const sha256_digest& input,
    preprocessor_graph_hook_module* const modules,
    const std::size_t module_count) noexcept {
    sha256 hash;
    constexpr char domain[] = "gore-as-external-hook-graph-output-v1\0";
    hash.update(domain, sizeof(domain) - 1U);
    hash.update(input.data(), input.size());
    hash_u64(hash, module_count);
    for (std::size_t index = 0U; index < module_count; ++index) {
        const lexical_module_description* const module = modules[index].module;
        hash_field(hash, module == nullptr ? std::string_view{} : module->module_name);
        hash_field(hash, modules[index].generated_declarations);
    }
    return hash.finish();
}

struct captured_hook_runtime {
    const external_frontend_profile* profile = nullptr;
    qualification_trace* trace = nullptr;
};

std::vector<std::string> nonempty_declaration(const std::string& value) {
    if (value.empty()) return {};
    return {value};
}

// BuildID 24539464 binds exactly one ClassAnalyze callback at target RVA
// 0x5775610.  Its complete semantic body is:
//
//   if (ClassDesc->CodeSuperClass->IsChildOf(UGameStateSubsystem::StaticClass())) {
//     bHasStatics = true;
//     GeneratedStatics += FString::Printf(
//       TEXT("\n %s Get() __generated {return Cast<%s>(GameStateSubsystem::"
//            "GetGameStateSubsystem(%s.Get()));}"),
//       *ClassName, *ClassName, *StaticClassGlobalVariableName);
//   }
//
// The profile's game_state_subsystem bit is derived from the same captured UClass
// ancestry, so new source classes do not depend on an exact source-text replay row.
bool captured_class_analyze(
    void* const context,
    const preprocessor_source& source,
    preprocessed_class_description& description,
    std::string& generated_statics,
    bool& has_statics,
    std::string& detail) noexcept {
    try {
        const auto* const runtime = static_cast<const captured_hook_runtime*>(context);
        if (runtime == nullptr || runtime->profile == nullptr ||
            !runtime->profile->class_analyze_bound) {
            detail = "ClassAnalyze is not bound by the sealed frontend profile";
            return false;
        }
        const sha256_digest source_digest = sha256_bytes(source.code.data(), source.code.size());
        const sha256_digest generated_digest =
            sha256_bytes(generated_statics.data(), generated_statics.size());
        const auto found = std::find_if(
            runtime->profile->class_analyze_captures.begin(),
            runtime->profile->class_analyze_captures.end(),
            [&](const class_analyze_capture& capture) {
                return capture.module_name == source.module_name &&
                    capture.name_space == description.name_space &&
                    capture.class_name == description.class_name &&
                    capture.source_sha256 == source_digest &&
                    capture.input_generated_statics_sha256 == generated_digest;
            });
        const std::string input_generated_statics = generated_statics;
        const bool input_has_statics = has_statics;
        const std::string input_compose_onto = description.compose_onto_class;
        apply_target_class_analyze_v24539464(description, generated_statics, has_statics);
        if (found != runtime->profile->class_analyze_captures.end() &&
            (found->generated_statics != generated_statics ||
             found->has_statics != has_statics ||
             found->compose_onto_class != description.compose_onto_class)) {
            generated_statics = input_generated_statics;
            has_statics = input_has_statics;
            description.compose_onto_class = input_compose_onto;
            detail = "captured ClassAnalyze output disagrees with the reversed target callback";
            return false;
        }
        if (runtime->trace != nullptr && !generated_statics.empty()) {
            std::string subject = source.module_name + "::";
            if (!description.name_space.empty()) subject += description.name_space + "::";
            subject += description.class_name;
            runtime->trace->class_analyze.push_back({
                std::move(subject), nonempty_declaration(generated_statics)});
            runtime->trace->generated_declarations.push_back(generated_statics);
        }
        detail.clear();
        return true;
    } catch (...) {
        try { detail = "ClassAnalyze capture lookup failed"; } catch (...) {}
        return false;
    }
}

bool apply_graph_capture(
    const graph_hook_profile& profile,
    preprocessor_graph_hook_module* const modules,
    const std::size_t module_count,
    std::vector<qualification_hook_capture>* const trace,
    std::vector<std::string>* const generated_trace,
    std::string& detail) {
    const sha256_digest input = graph_input_digest(modules, module_count);
    const auto found = std::find_if(
        profile.captures.begin(), profile.captures.end(),
        [&](const graph_hook_capture& capture) {
            return capture.input_graph_sha256 == input;
        });
    if (found == profile.captures.end()) {
        detail = "no exact module-graph capture matches this hook input";
        return false;
    }
    if (found->modules.size() != module_count) {
        detail = "captured hook output does not cover the complete module graph";
        return false;
    }
    for (std::size_t index = 0U; index < module_count; ++index) {
        if (modules[index].module == nullptr ||
            found->modules[index].module_name != modules[index].module->module_name) {
            detail = "captured hook module order does not match the input graph";
            return false;
        }
        modules[index].generated_declarations =
            found->modules[index].generated_declarations;
        if (trace != nullptr && !found->modules[index].generated_declarations.empty()) {
            trace->push_back({found->modules[index].module_name,
                nonempty_declaration(found->modules[index].generated_declarations)});
        }
        if (generated_trace != nullptr &&
            !found->modules[index].generated_declarations.empty()) {
            generated_trace->push_back(found->modules[index].generated_declarations);
        }
    }
    if (graph_output_digest(input, modules, module_count) != found->output_graph_sha256) {
        detail = "captured hook output digest does not match its declarations";
        return false;
    }
    detail.clear();
    return true;
}

bool captured_process_chunks(
    void* const context,
    preprocessor_graph_hook_module* const modules,
    const std::size_t module_count,
    std::string& detail) noexcept {
    try {
        const auto* const runtime = static_cast<const captured_hook_runtime*>(context);
        if (runtime == nullptr || runtime->profile == nullptr ||
            !runtime->profile->process_chunks.bound) {
            detail = "OnProcessChunks is not bound by the sealed frontend profile";
            return false;
        }
        return apply_graph_capture(
            runtime->profile->process_chunks, modules, module_count,
            runtime->trace == nullptr ? nullptr : &runtime->trace->process_chunks,
            runtime->trace == nullptr ? nullptr : &runtime->trace->generated_declarations,
            detail);
    } catch (...) {
        try { detail = "OnProcessChunks capture lookup failed"; } catch (...) {}
        return false;
    }
}

bool captured_post_process_code(
    void* const context,
    preprocessor_graph_hook_module* const modules,
    const std::size_t module_count,
    std::string& detail) noexcept {
    try {
        const auto* const runtime = static_cast<const captured_hook_runtime*>(context);
        if (runtime == nullptr || runtime->profile == nullptr ||
            !runtime->profile->post_process_code.bound) {
            detail = "OnPostProcessCode is not bound by the sealed frontend profile";
            return false;
        }
        return apply_graph_capture(
            runtime->profile->post_process_code, modules, module_count,
            runtime->trace == nullptr ? nullptr : &runtime->trace->post_process,
            runtime->trace == nullptr ? nullptr : &runtime->trace->generated_declarations,
            detail);
    } catch (...) {
        try { detail = "OnPostProcessCode capture lookup failed"; } catch (...) {}
        return false;
    }
}

bool script_source_path(const std::string_view path) noexcept {
    return path.size() >= 3U && path[path.size() - 3U] == '.' &&
        std::tolower(static_cast<unsigned char>(path[path.size() - 2U])) == 'a' &&
        std::tolower(static_cast<unsigned char>(path[path.size() - 1U])) == 's';
}

bool skipped_script_directory(const std::string_view path) noexcept {
    std::size_t begin = 0U;
    while (begin < path.size()) {
        const std::size_t end = path.find('/', begin);
        if (end == std::string_view::npos) return false;
        const std::string_view component = path.substr(begin, end - begin);
        if (component == "Editor" || component == "Dev" || component == "Examples") return true;
        begin = end + 1U;
    }
    return false;
}

std::string source_module_name(std::string path) {
    for (std::size_t position = 0U; position + 2U < path.size();) {
        if (path[position] == '.' &&
            std::tolower(static_cast<unsigned char>(path[position + 1U])) == 'a' &&
            std::tolower(static_cast<unsigned char>(path[position + 2U])) == 's') {
            path.erase(position, 3U);
        } else {
            ++position;
        }
    }
    std::replace(path.begin(), path.end(), '/', '.');
    return path;
}

void message_callback(const asSMessageInfo* message, void* parameter) {
    auto& diagnostics = *static_cast<std::vector<compiler_diagnostic>*>(parameter);
    if (message == nullptr || diagnostics.size() >= wire::kMaxDiagnostics) return;
    const std::string text = message->message == nullptr ? "" : message->message;
    // The game hook records only non-empty messages.
    if (text.empty()) return;
    // The game hook deliberately suppresses AngelScript's per-function progress
    // chatter. It is not a source diagnostic and must not enter parity output.
    if (message->type == asMSGTYPE_INFORMATION &&
        text.compare(0U, 10U, "Compiling ") == 0) {
        return;
    }
    const char* severity = message->type == asMSGTYPE_ERROR ? "error" :
        (message->type == asMSGTYPE_WARNING ? "warning" : "info");
    std::string source = message->section == nullptr ? "" : message->section;
    // The capture hook labels every global AngelScript diagnostic this way;
    // this is not specific to warnings-as-errors.
    if (source.empty()) source = "(?)";
    diagnostics.push_back({
        severity,
        message->type == asMSGTYPE_ERROR ? "GORE_AS_COMPILER_ERROR" :
            (message->type == asMSGTYPE_WARNING ? "GORE_AS_COMPILER_WARNING" : "GORE_AS_COMPILER_INFO"),
        text,
        std::move(source),
        message->row < 0 ? 0U : static_cast<std::uint32_t>(message->row),
        message->col < 0 ? 0U : static_cast<std::uint32_t>(message->col),
    });
}

bool qualification_safe_primitive_opcode(const asBYTE opcode) noexcept {
    // Closed VM subset. Anything that can call, address globals, or touch objects, pointers,
    // references, allocation, or strings is rejected before execution can reach inert host stubs.
    switch (opcode) {
    case asBC_PshC4: case asBC_PshV4: case asBC_NOT: case asBC_RET:
    case asBC_JMP: case asBC_JZ: case asBC_JNZ: case asBC_JS: case asBC_JNS:
    case asBC_JP: case asBC_JNP: case asBC_TZ: case asBC_TNZ: case asBC_TS:
    case asBC_TNS: case asBC_TP: case asBC_TNP:
    case asBC_NEGi: case asBC_NEGf: case asBC_NEGd:
    case asBC_INCi16: case asBC_INCi8: case asBC_DECi16: case asBC_DECi8:
    case asBC_INCi: case asBC_DECi: case asBC_INCf: case asBC_DECf:
    case asBC_INCd: case asBC_DECd: case asBC_IncVi: case asBC_DecVi:
    case asBC_BNOT: case asBC_BAND: case asBC_BOR: case asBC_BXOR:
    case asBC_BSLL: case asBC_BSRL: case asBC_BSRA: case asBC_PshC8:
    case asBC_CMPd: case asBC_CMPu: case asBC_CMPf: case asBC_CMPi:
    case asBC_CMPIi: case asBC_CMPIf: case asBC_CMPIu: case asBC_SUSPEND:
    case asBC_SetV4: case asBC_SetV8: case asBC_ADDSi:
    case asBC_CpyVtoV4: case asBC_CpyVtoV8: case asBC_CpyVtoR4: case asBC_CpyVtoR8:
    case asBC_CpyRtoV4: case asBC_CpyRtoV8: case asBC_iTOf: case asBC_fTOi:
    case asBC_uTOf: case asBC_fTOu: case asBC_sbTOi: case asBC_swTOi:
    case asBC_ubTOi: case asBC_uwTOi: case asBC_dTOi: case asBC_dTOu:
    case asBC_dTOf: case asBC_iTOd: case asBC_uTOd: case asBC_fTOd:
    case asBC_ADDi: case asBC_SUBi: case asBC_MULi: case asBC_DIVi: case asBC_MODi:
    case asBC_ADDf: case asBC_SUBf: case asBC_MULf: case asBC_DIVf: case asBC_MODf:
    case asBC_ADDd: case asBC_SUBd: case asBC_MULd: case asBC_DIVd: case asBC_MODd:
    case asBC_ADDIi: case asBC_SUBIi: case asBC_MULIi:
    case asBC_ADDIf: case asBC_SUBIf: case asBC_MULIf:
    case asBC_iTOb: case asBC_iTOw: case asBC_SetV1: case asBC_SetV2:
    case asBC_i64TOi: case asBC_uTOi64: case asBC_iTOi64: case asBC_fTOi64:
    case asBC_dTOi64: case asBC_fTOu64: case asBC_dTOu64: case asBC_i64TOf:
    case asBC_u64TOf: case asBC_i64TOd: case asBC_u64TOd: case asBC_NEGi64:
    case asBC_INCi64: case asBC_DECi64: case asBC_BNOT64:
    case asBC_ADDi64: case asBC_SUBi64: case asBC_MULi64: case asBC_DIVi64:
    case asBC_MODi64: case asBC_BAND64: case asBC_BOR64: case asBC_BXOR64:
    case asBC_BSLL64: case asBC_BSRL64: case asBC_BSRA64:
    case asBC_CMPi64: case asBC_CMPu64: case asBC_ClrHi: case asBC_JitEntry:
    case asBC_PshV8: case asBC_DIVu: case asBC_MODu: case asBC_JLowZ:
    case asBC_JLowNZ: case asBC_POWi: case asBC_POWu: case asBC_POWf:
    case asBC_POWd: case asBC_POWdi: case asBC_POWi64: case asBC_POWu64:
        return true;
    default:
        return false;
    }
}

qualification_runtime_kind qualification_runtime_for(
    const qualification_request& request) noexcept {
    if (request.case_id == "positive.invoke.structured" &&
        request.invoke_declaration == "TArray<int32> QualificationInvokeArray()") {
        return qualification_runtime_kind::t_array_int32;
    }
    if ((request.case_id == "positive.fname.non-ascii-equivalence" &&
            request.invoke_declaration == "bool QualificationFNameEquivalent()") ||
        (request.case_id == "positive.fname.name-none-canonical" &&
            request.invoke_declaration == "bool QualificationFNameNoneCanonical()")) {
        return qualification_runtime_kind::fname_equivalence;
    }
    if (request.case_id == "positive.strings.factory-roundtrip" &&
        request.invoke_declaration == "FString QualificationStringRoundtrip()") {
        return qualification_runtime_kind::fstring_roundtrip;
    }
    return qualification_runtime_kind::none;
}

bool qualification_invoke(
    asIScriptEngine& engine,
    const std::vector<asIScriptModule*>& modules,
    const qualification_request& request,
    const registry_runtime& registry_runtime_state,
    qualification_trace& trace,
    std::string& detail) {
    if (request.invoke_declaration.empty()) return true;
    asIScriptModule* selected = nullptr;
    for (asIScriptModule* const module : modules) {
        if (module != nullptr && request.invoke_module == module->GetName()) {
            if (selected != nullptr) {
                detail = "qualification invoke module identity is ambiguous";
                return false;
            }
            selected = module;
        }
    }
    asIScriptFunction* const function = selected == nullptr ? nullptr :
        selected->GetFunctionByDecl(request.invoke_declaration.c_str());
    if (function == nullptr || function->GetParamCount() != 0U) {
        detail = "qualification invoke declaration did not resolve to one zero-argument script function";
        return false;
    }
    const int type_id = function->GetReturnTypeId();
    const qualification_runtime_kind runtime_kind = registry_runtime_state.qualification_kind();
    const bool primitive_return = type_id >= asTYPEID_BOOL && type_id <= asTYPEID_LAST_PRIMITIVE;
    if (!primitive_return && (runtime_kind == qualification_runtime_kind::none ||
        !registry_runtime_state.qualification_object_type_allowed(engine.GetTypeInfoById(type_id)))) {
        detail = "qualification invoke return requires unavailable sealed host-object semantics";
        return false;
    }
    asUINT word_count = 0U;
    const asDWORD* const bytecode = function->GetByteCode(&word_count);
    if (bytecode == nullptr || word_count == 0U) {
        detail = "qualification invoke function has no inspectable bytecode";
        return false;
    }
    std::size_t offset = 0U;
    while (offset < word_count) {
        const asBYTE opcode = *reinterpret_cast<const asBYTE*>(bytecode + offset);
        if ((runtime_kind == qualification_runtime_kind::none &&
                !qualification_safe_primitive_opcode(opcode)) ||
            (runtime_kind != qualification_runtime_kind::none &&
                !registry_runtime_state.qualification_instruction_allowed(
                    engine, bytecode + offset, opcode, detail))) {
            if (detail.empty()) {
                detail = "qualification invoke bytecode escapes its sealed runtime subset";
            }
            return false;
        }
        const int words = asBCTypeSize[asBCInfo[opcode].type];
        if (words <= 0 || offset + static_cast<std::size_t>(words) > word_count) {
            detail = "qualification invoke bytecode is not instruction aligned";
            return false;
        }
        offset += static_cast<std::size_t>(words);
    }
    asIScriptContext* const context = engine.CreateContext();
    if (context == nullptr) { detail = "qualification invoke context allocation failed"; return false; }
    const bool prepared = context->Prepare(function) >= 0;
    const int executed = prepared ? context->Execute() : asERROR;
    if (!prepared || executed != asEXECUTION_FINISHED) {
        detail = "qualification invoke did not finish normally";
        context->Release();
        return false;
    }
    trace.has_invoke_return = true;
    if (!primitive_return) {
        const void* const object = context->GetReturnObject();
        if (runtime_kind == qualification_runtime_kind::t_array_int32) {
            std::vector<std::int32_t> values;
            if (!registry_runtime_state.read_qualification_tarray_int32(object, values, detail)) {
                context->Release();
                return false;
            }
            trace.invoke_type = "TArray<int32>";
            trace.invoke_kind = "sequence";
            trace.invoke_value_json = "[";
            for (std::size_t index = 0U; index < values.size(); ++index) {
                if (index != 0U) trace.invoke_value_json += ',';
                trace.invoke_value_json += "{\"kind\":\"i64\",\"value\":";
                trace.invoke_value_json += std::to_string(values[index]);
                trace.invoke_value_json += '}';
            }
            trace.invoke_value_json += ']';
        } else if (runtime_kind == qualification_runtime_kind::fstring_roundtrip) {
            std::string value;
            if (!registry_runtime_state.read_qualification_fstring(object, value, detail)) {
                context->Release();
                return false;
            }
            trace.invoke_type = "FString";
            trace.invoke_kind = "utf8";
            trace.invoke_value_json = "\"" + json_escape(value) + "\"";
        } else {
            detail = "qualification object return escaped its sealed runtime adapter";
            context->Release();
            return false;
        }
        context->Release();
        return true;
    }
    switch (type_id) {
    case asTYPEID_BOOL:
        trace.invoke_type = "bool"; trace.invoke_kind = "bool";
        trace.invoke_value_json = context->GetReturnByte() == 0U ? "false" : "true"; break;
    case asTYPEID_INT8:
        trace.invoke_type = "int8"; trace.invoke_kind = "i64";
        trace.invoke_value_json = std::to_string(static_cast<std::int8_t>(context->GetReturnByte())); break;
    case asTYPEID_INT16:
        trace.invoke_type = "int16"; trace.invoke_kind = "i64";
        trace.invoke_value_json = std::to_string(static_cast<std::int16_t>(context->GetReturnWord())); break;
    case asTYPEID_INT32:
        trace.invoke_type = "int32"; trace.invoke_kind = "i64";
        trace.invoke_value_json = std::to_string(static_cast<std::int32_t>(context->GetReturnDWord())); break;
    case asTYPEID_INT64:
        trace.invoke_type = "int64"; trace.invoke_kind = "i64";
        trace.invoke_value_json = std::to_string(static_cast<std::int64_t>(context->GetReturnQWord())); break;
    case asTYPEID_UINT8:
        trace.invoke_type = "uint8"; trace.invoke_kind = "u64";
        trace.invoke_value_json = std::to_string(context->GetReturnByte()); break;
    case asTYPEID_UINT16:
        trace.invoke_type = "uint16"; trace.invoke_kind = "u64";
        trace.invoke_value_json = std::to_string(context->GetReturnWord()); break;
    case asTYPEID_UINT32:
        trace.invoke_type = "uint32"; trace.invoke_kind = "u64";
        trace.invoke_value_json = std::to_string(context->GetReturnDWord()); break;
    case asTYPEID_UINT64:
        trace.invoke_type = "uint64"; trace.invoke_kind = "u64";
        trace.invoke_value_json = std::to_string(context->GetReturnQWord()); break;
    case asTYPEID_FLOAT32: {
        std::uint32_t bits = 0U; const float value = context->GetReturnFloat();
        std::memcpy(&bits, &value, sizeof(bits));
        trace.invoke_type = "float"; trace.invoke_kind = "f32_bits";
        trace.invoke_value_json = std::to_string(bits); break;
    }
    case asTYPEID_FLOAT64: {
        std::uint64_t bits = 0U; const double value = context->GetReturnDouble();
        std::memcpy(&bits, &value, sizeof(bits));
        trace.invoke_type = "double"; trace.invoke_kind = "f64_bits";
        trace.invoke_value_json = std::to_string(bits); break;
    }
    default:
        detail = "qualification invoke return type escaped the primitive gate";
        context->Release();
        return false;
    }
    context->Release();
    return true;
}

} // namespace

void apply_target_class_analyze_v24539464(
    const preprocessed_class_description& description,
    std::string& generated_statics,
    bool& has_statics) {
    if (!description.code_super_game_state_subsystem) return;
    const std::string& name = description.class_name;
    generated_statics += "\n " + name +
        " Get() __generated {return Cast<" + name +
        ">(GameStateSubsystem::GetGameStateSubsystem(" +
        description.static_class_global_variable_name + ".Get()));}";
    has_statics = true;
}

sidecar_compile_result compile_sidecar_request(
    const std::wstring_view request_path,
    const bool allow_qualification) noexcept {
    try {
        std::string detail;
        if (request_path.empty() || request_path.size() > wire::kMaxRequestPathUtf16Units) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_REQUEST_PATH_INVALID", "request path is empty or exceeds the protocol limit");
        }
        std::vector<std::uint8_t> request_bytes;
        if (!read_file(std::wstring(request_path), wire::kMaxRequestBytes, request_bytes, detail)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_REQUEST_READ_FAILED", detail);
        }
        if (request_bytes.empty()) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_REQUEST_EMPTY", "request file is empty");
        }
        compile_request request;
        const std::string request_text(reinterpret_cast<const char*>(request_bytes.data()), request_bytes.size());
        if (!parse_request(request_text, request, detail, allow_qualification)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_REQUEST_INVALID", detail);
        }
        const std::size_t output_separator = request.output_path.find_last_of(L'\\');
        const std::wstring output_parent = output_separator == std::wstring::npos
            ? std::wstring{}
            : request.output_path.substr(0U, output_separator);
        if (!inspect_directory(request.profile_root, detail) ||
            !inspect_directory(request.source_root, detail) || output_parent.empty() ||
            !inspect_directory(output_parent, detail) ||
            !path_below(request.profile_root, request.manifest_path)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_REQUEST_DIRECTORY_INVALID", detail.empty()
                ? "manifest path is not below the sealed profile root" : detail);
        }

        std::string manifest_text;
        if (!read_text(request.manifest_path, 4U * 1024U * 1024U, manifest_text, detail)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_PROFILE_MANIFEST_READ_FAILED", detail);
        }
        compiler_profile_manifest manifest;
        if (!parse_compiler_profile_manifest(
                manifest_text, manifest, detail, !request.qualification_mode)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_PROFILE_INVALID", detail);
        }
        if (manifest.profile_sha256 != request.profile_sha256 ||
            manifest.steam_build_id != request.steam_build_id || manifest.depot_id != request.depot_id ||
            manifest.depot_manifest_gid != request.depot_manifest_gid ||
            manifest.required_probe_suite_version != request.required_probe_suite_version ||
            (request.qualification_mode &&
             manifest.required_probe_suite_version != request.qualification.suite_id)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_PROFILE_IDENTITY_MISMATCH",
                "request and compiler manifest identities differ");
        }
        if (request.base_cache.byte_len != manifest.oracle_shipping_cache.byte_len ||
            request.base_cache.sha256 != manifest.oracle_shipping_cache.sha256 ||
            request.binds_cache.byte_len != manifest.oracle_binds_cache.byte_len ||
            request.binds_cache.sha256 != manifest.oracle_binds_cache.sha256) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_ORACLE_INPUT_MISMATCH",
                "base cache or Binds.Cache seal does not match the qualified profile");
        }

        std::map<std::string, std::vector<std::uint8_t>> blobs;
        std::uint64_t aggregate = 0U;
        for (const auto& blob : manifest.all_blobs) {
            if (blobs.count(blob.path) != 0U) continue;
            if (blob.byte_len > max_profile_blob_bytes || aggregate > max_profile_aggregate_bytes - blob.byte_len) {
                return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_PROFILE_BLOB_LIMIT",
                    "profile blob set exceeds the native limit");
            }
            const std::wstring path = joined(request.profile_root, blob.path, detail);
            std::vector<std::uint8_t> bytes;
            if (path.empty() || !read_file(path, max_profile_blob_bytes, bytes, detail, &blob.byte_len, &blob.sha256)) {
                return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_PROFILE_BLOB_INVALID", detail);
            }
            aggregate += blob.byte_len;
            blobs.emplace(blob.path, std::move(bytes));
        }
        const auto text_blob = [&](const sealed_blob& blob) -> std::string_view {
            const auto& bytes = blobs.at(blob.path);
            return {reinterpret_cast<const char*>(bytes.data()), bytes.size()};
        };
        registry_profile registry;
        if (!parse_registry_profile_payloads(
                text_blob(manifest.ordered_engine_properties), text_blob(manifest.registration_trace),
                text_blob(manifest.post_bind_snapshot), manifest.registration_trace_count,
                registry, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_REGISTRY_PROFILE_INVALID", detail);
        }
        preprocessor_options preprocessor;
        compiler_options options;
        external_frontend_profile external_frontend;
        if (!parse_frontend_profile_payloads(
                text_blob(manifest.preprocessor_config), text_blob(manifest.class_generator_config),
                text_blob(manifest.compiler_options), preprocessor, options,
                external_frontend, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_FRONTEND_PROFILE_INVALID", detail);
        }
        if (external_frontend.process_chunks.bound ||
            external_frontend.post_process_code.bound ||
            !external_frontend.process_chunks.captures.empty() ||
            !external_frontend.post_process_code.captures.empty()) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable",
                "GORE_AS_FRONTEND_TARGET_MISMATCH",
                "BuildID 24539464 has no bound ProcessChunks or PostProcessCode delegate");
        }

        std::vector<std::uint8_t> base_bytes;
        if (!read_file(request.base_cache.path, max_base_cache_bytes, base_bytes, detail,
                &request.base_cache.byte_len, &request.base_cache.sha256)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_BASE_CACHE_INVALID", detail);
        }
        std::vector<std::uint8_t> binds_bytes;
        if (!read_file(request.binds_cache.path, max_binds_cache_bytes, binds_bytes, detail,
                &request.binds_cache.byte_len, &request.binds_cache.sha256)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_BINDS_CACHE_INVALID", detail);
        }
        (void)binds_bytes;
        cache_wire::cache base;
        cache_wire::codec_error codec;
        if (!cache_wire::decode(base_bytes.data(), base_bytes.size(), base, codec)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_BASE_CACHE_DECODE_FAILED",
                codec.field + " at " + std::to_string(codec.offset) + ": " + codec.detail);
        }
        if (!validate_and_apply_full_graph(request, base, detail)) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_FULL_GRAPH_INVALID", detail);
        }

        std::map<std::string, std::string> source_contents;
        for (const auto& file : request.source_files) {
            const std::wstring path = joined(request.source_root, file.relative_path, detail);
            std::vector<std::uint8_t> bytes;
            if (path.empty() || !read_file(path, wire::kMaxSourceFileBytes, bytes, detail, &file.byte_len, &file.sha256)) {
                return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_SOURCE_FILE_INVALID", detail);
            }
            std::string code(reinterpret_cast<const char*>(bytes.data()), bytes.size());
            const bool discovered_script = script_source_path(file.relative_path) &&
                (preprocessor.use_editor_scripts || !skipped_script_directory(file.relative_path));
            if (discovered_script &&
                (code.find('\0') != std::string::npos || !is_valid_utf8(code))) {
                return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_SOURCE_ENCODING_INVALID",
                    "discovered .as source must be canonical UTF-8 without NUL bytes");
            }
            source_contents.emplace(file.relative_path, std::move(code));
        }
        if (request.qualification_mode && !validate_qualification_corpus_request(
                text_blob(manifest.codegen_probe_corpus), request, source_contents, detail)) {
            return failure(wire::ExitCode::data_error, "rejected",
                "GORE_AS_QUALIFICATION_CORPUS_MISMATCH", detail);
        }
        const std::vector<preprocessor_base_module> base_descriptors = base_modules(base);
        std::set<std::string> base_module_names;
        for (const preprocessor_base_module& module : base_descriptors) {
            base_module_names.insert(module.module_name);
        }
        std::map<std::string, const overlay_module*> overlay_by_path;
        for (const overlay_module& overlay : request.overlays) {
            overlay_by_path.emplace(overlay.relative_path, &overlay);
        }
        std::map<std::string, const graph_change*> change_by_path;
        for (const graph_change& change : request.changes) {
            if (change.operation != graph_change_operation::remove) {
                change_by_path.emplace(change.relative_path, &change);
            }
        }
        std::set<std::string> resolved_overlays;
        std::vector<preprocessor_source> sources;
        sources.reserve(request.source_files.size());
        for (const source_file& file : request.source_files) {
            const auto requested = overlay_by_path.find(file.relative_path);
            const auto graph_requested = change_by_path.find(file.relative_path);
            if (!script_source_path(file.relative_path)) {
                if (requested != overlay_by_path.end() ||
                    graph_requested != change_by_path.end()) {
                    return failure(wire::ExitCode::data_error, "rejected",
                        "GORE_AS_SOURCE_DISCOVERY_INVALID",
                        "declared source path is not a donor-discoverable .as source file");
                }
                continue;
            }
            if (!preprocessor.use_editor_scripts && skipped_script_directory(file.relative_path)) {
                if (requested != overlay_by_path.end() ||
                    graph_requested != change_by_path.end()) {
                    return failure(wire::ExitCode::data_error, "rejected",
                        "GORE_AS_EDITOR_SOURCE_DISABLED",
                        "profile disables Editor, Dev, and Examples source directories");
                }
                continue;
            }
            const std::string module_name = source_module_name(file.relative_path);
            const preprocessor_source::operation inferred_operation =
                base_module_names.count(module_name) == 0U
                    ? preprocessor_source::operation::add
                    : preprocessor_source::operation::edit;
            if (requested != overlay_by_path.end()) {
                if (requested->second->module_name != module_name ||
                    requested->second->operation != inferred_operation) {
                    return failure(wire::ExitCode::data_error, "rejected",
                        "GORE_AS_SOURCE_DISCOVERY_MISMATCH",
                        "overlay identity/operation disagrees with the sealed source path and pristine cache");
                }
                resolved_overlays.insert(file.relative_path);
            } else if (full_graph_request(request)) {
                const auto expected_operation = inferred_operation == preprocessor_source::operation::add
                    ? graph_change_operation::add : graph_change_operation::edit;
                if (graph_requested == change_by_path.end() ||
                    graph_requested->second->module_name != module_name ||
                    graph_requested->second->operation != expected_operation) {
                    return failure(wire::ExitCode::data_error, "rejected",
                        "GORE_AS_SOURCE_DISCOVERY_MISMATCH",
                        "FullGraph source identity/operation disagrees with the sealed path and pruned base cache");
                }
                resolved_overlays.insert(file.relative_path);
            }
            preprocessor_source source;
            source.relative_path = file.relative_path;
            source.absolute_path = request.source_root_utf8 + "/" + file.relative_path;
            source.code = source_contents.at(file.relative_path);
            source.overlay_operation = inferred_operation;
            source.module_name = module_name;
            sources.push_back(std::move(source));
        }
        const std::size_t requested_source_count =
            full_graph_request(request)
                ? change_by_path.size() : request.overlays.size();
        if (resolved_overlays.size() != requested_source_count ||
            (request.request_version == wire::kRequestProtocolVersionV1 && sources.empty())) {
            return failure(wire::ExitCode::data_error, "rejected",
                "GORE_AS_SOURCE_DISCOVERY_INCOMPLETE",
                "not every requested source resolves into the enabled sealed source graph");
        }
        preprocessor.static_names.reserve(base.static_names.size());
        for (const auto& name : base.static_names) preprocessor.static_names.push_back(name.bytes);
        qualification_trace qualification_evidence;
        qualification_evidence.class_analyze_bound = external_frontend.class_analyze_bound;
        qualification_evidence.process_chunks_bound = external_frontend.process_chunks.bound;
        qualification_evidence.post_process_code_bound = external_frontend.post_process_code.bound;
#if defined(AS_REFERENCE_DEBUGGING) && AS_REFERENCE_DEBUGGING
        qualification_evidence.as_reference_debugging = true;
#endif
        // The standalone Shipping reconstruction intentionally never registers the UE5 editor
        // object-pointer resolver. This records effective runtime state, not a target label.
        qualification_evidence.resolve_object_ptr_callback_registered =
            qualification_resolve_object_ptr_callback_registered;
        captured_hook_runtime captured_runtime{
            &external_frontend,
            request.qualification_mode ? &qualification_evidence : nullptr};
        preprocessor_hooks hooks;
        hooks.context = &captured_runtime;
        hooks.class_analyze = external_frontend.class_analyze_bound
            ? &captured_class_analyze : nullptr;
        hooks.process_chunks = external_frontend.process_chunks.bound
            ? &captured_process_chunks : nullptr;
        hooks.post_process_code = external_frontend.post_process_code.bound
            ? &captured_post_process_code : nullptr;
        const auto preprocessing = preprocess_lexical_module_graph(
            preprocessor, sources, base_descriptors, &hooks);
        if (request.qualification_mode) {
            for (const lexical_module_description& module : preprocessing.modules) {
                if (!module.editor_only_blocks.empty()) {
                    qualification_evidence.editor_discovery.push_back(module.module_name);
                }
                const auto source = std::find_if(
                    sources.begin(), sources.end(), [&](const preprocessor_source& candidate) {
                        return candidate.module_name == module.module_name &&
                            candidate.code.find("#if RELEASE") != std::string::npos;
                    });
                const bool retained_development_code = std::any_of(
                    module.code.begin(), module.code.end(), [](const preprocessed_code_section& section) {
                        return section.conditioned_code.find("QualificationReleaseDiscovery") !=
                            std::string::npos;
                    });
                if (source != sources.end() && retained_development_code) {
                    qualification_evidence.release_discovery.push_back(module.module_name);
                }
            }
            const auto canonicalize = [](std::vector<std::string>& values) {
                std::sort(values.begin(), values.end());
                values.erase(std::unique(values.begin(), values.end()), values.end());
            };
            canonicalize(qualification_evidence.generated_declarations);
            canonicalize(qualification_evidence.editor_discovery);
            canonicalize(qualification_evidence.release_discovery);
        }
        std::vector<compiler_diagnostic> diagnostics;
        for (const auto& diagnostic : preprocessing.diagnostics) {
            diagnostics.push_back({
                diagnostic.severity == preprocessor_diagnostic_severity::error ? "error" : "warning",
                diagnostic.severity == preprocessor_diagnostic_severity::error ?
                    "GORE_AS_PREPROCESSOR_ERROR" : "GORE_AS_PREPROCESSOR_WARNING",
                diagnostic.message, diagnostic.absolute_path, diagnostic.row, diagnostic.column});
        }
        if (!preprocessing.ok) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_PREPROCESSOR_REJECTED",
                "source preprocessing failed", diagnostics);
        }

        FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode = options.error_on_incorrect_editor_only_code;
        FAngelscriptManager::Get().ConfigSettings->bWarnOnDivergentComparisonOperatorOverloads =
            options.warn_on_divergent_comparison_operator_overloads;
        FAngelscriptManager::Get().ConfigSettings->bWarnOnImplicitSignedUnsignedConversion =
            options.warn_on_implicit_signed_unsigned_conversion;
        FAngelscriptManager::Get().ConfigSettings->bWarnOnIncrementDecrementInComplexExpression =
            options.warn_on_increment_decrement_in_complex_expression;
        FAngelscriptManager::Get().ConfigSettings->bWarnOnUnusedReturnValueForConstMethods =
            options.warn_on_unused_return_value_for_const_methods;

        registry_runtime registry_runtime_state;
        const qualification_runtime_kind qualification_runtime = request.qualification_mode
            ? qualification_runtime_for(request.qualification)
            : qualification_runtime_kind::none;
        if (qualification_runtime != qualification_runtime_kind::none &&
            !registry_runtime_state.configure_qualification_runtime(
                qualification_runtime, preprocessing.static_names,
                preprocessing.static_name_comparison_identities, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable",
                "GORE_AS_QUALIFICATION_RUNTIME_UNAVAILABLE", detail, diagnostics);
        }
        frontend_compile_runtime frontend_runtime;
        engine_ptr engine(asCreateScriptEngine(manifest.as_create_version));
        if (!engine) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_ENGINE_CREATE_FAILED",
                "the pinned AngelScript engine rejected the profile version");
        }
        engine->SetMessageCallback(asFUNCTION(message_callback), &diagnostics, asCALL_CDECL);
        const auto replay = replay_registry(*engine, registry, registry_runtime_state);
        if (!replay.succeeded()) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_REGISTRY_REPLAY_FAILED",
                replay.detail + " (ordinal " + std::to_string(replay.failed_ordinal) + ")");
        }
        if (qualification_runtime != qualification_runtime_kind::none &&
            !registry_runtime_state.qualification_runtime_ready(detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable",
                "GORE_AS_QUALIFICATION_RUNTIME_UNAVAILABLE", detail, diagnostics);
        }
        std::vector<asIScriptModule*> modules;
        const auto compiled = cache_wire::compile_mixed_cache_checkpoint(
            *engine, base, preprocessor, preprocessing, &registry_runtime_state, frontend_runtime, modules);
        if (!compiled.succeeded()) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_COMPILE_REJECTED",
                compiled.detail, diagnostics);
        }
        if (qualification_runtime != qualification_runtime_kind::none &&
            !registry_runtime_state.prepare_qualification_runtime(*engine, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable",
                "GORE_AS_QUALIFICATION_RUNTIME_UNAVAILABLE", detail, diagnostics);
        }
        if (request.qualification_mode && !qualification_invoke(
                *engine, modules, request.qualification, registry_runtime_state,
                qualification_evidence, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable",
                "GORE_AS_QUALIFICATION_INVOKE_UNAVAILABLE", detail, diagnostics);
        }
        const auto static_jit = cache_wire::apply_shipping_static_jit_checkpoint(modules);
        if (!static_jit.succeeded()) {
            return failure(
                wire::ExitCode::software, "invalid_output",
                "GORE_AS_STATIC_JIT_ANALYSIS_FAILED", static_jit.detail, diagnostics);
        }

        sha256 guid_hash;
        if (request.request_version == wire::kRequestProtocolVersionV1) {
            constexpr char guid_domain[] = "gore-as-output-guid-v1\0";
            guid_hash.update(guid_domain, sizeof(guid_domain) - 1U);
            guid_hash.update(request.profile_sha256.data(), request.profile_sha256.size());
            guid_hash.update(request.base_cache.sha256.data(), request.base_cache.sha256.size());
            for (const preprocessor_source& source : sources) {
                guid_hash.update(source.module_name);
                guid_hash.update(std::string_view("\0", 1U));
                guid_hash.update(source.relative_path);
                guid_hash.update(std::string_view("\0", 1U));
                const auto& file = *std::find_if(request.source_files.begin(), request.source_files.end(),
                    [&](const source_file& candidate) { return candidate.relative_path == source.relative_path; });
                guid_hash.update(file.sha256.data(), file.sha256.size());
            }
        } else {
            constexpr char guid_domain[] = "gore-as-output-guid-v2\0";
            guid_hash.update(guid_domain, sizeof(guid_domain) - 1U);
            guid_hash.update(request.profile_sha256.data(), request.profile_sha256.size());
            guid_hash.update(request.base_cache.sha256.data(), request.base_cache.sha256.size());
            guid_hash.update(request.binds_cache.sha256.data(), request.binds_cache.sha256.size());
            hash_u64(guid_hash, request.changes.size());
            for (const graph_change& change : request.changes) {
                const std::uint8_t operation_tag =
                    change.operation == graph_change_operation::add ? 1U :
                    (change.operation == graph_change_operation::edit ? 2U : 3U);
                guid_hash.update(&operation_tag, 1U);
                hash_field(guid_hash, change.module_name);
                hash_field(guid_hash, change.relative_path);
                const std::uint8_t has_source = change.has_source ? 1U : 0U;
                guid_hash.update(&has_source, 1U);
                if (change.has_source) {
                    hash_u64(guid_hash, change.source_byte_len);
                    guid_hash.update(change.source_sha256.data(), change.source_sha256.size());
                }
            }
            hash_u64(guid_hash, request.final_manifest.size());
            for (const final_module& module : request.final_manifest) {
                hash_field(guid_hash, module.module_name);
                hash_field(guid_hash, module.relative_path);
            }
        }
        const auto guid_digest = guid_hash.finish();
        std::array<std::uint8_t, 16U> guid{};
        std::copy_n(guid_digest.begin(), guid.size(), guid.begin());
        cache_wire::cache generated;
        const auto exported = cache_wire::export_mixed_graph_checkpoint(
            base, preprocessing, modules, guid, manifest.build_identifier, generated,
            registry_runtime_state,
            options.mark_non_uproperty_properties_as_transient);
        if (!exported.succeeded()) {
            if (exported.is_compile_diagnostic) {
                diagnostics.push_back({
                    "error", "GORE_AS_COMPILER_ERROR", exported.detail,
                    exported.diagnostic_source, exported.diagnostic_line,
                    exported.diagnostic_column});
                return failure(
                    wire::ExitCode::data_error, "rejected",
                    "GORE_AS_COMPILE_REJECTED", exported.detail, diagnostics);
            }
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_CACHE_EXPORT_FAILED", exported.detail);
        }
        if (!validate_full_graph_output(request, generated, detail)) {
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_FULL_GRAPH_OUTPUT_MISMATCH", detail);
        }
        cache_wire::cache qualification_projection;
        const cache_wire::cache* response_cache = &generated;
        if (request.qualification_mode) {
            const auto projected = cache_wire::export_source_graph_checkpoint(
                base, preprocessing, modules, guid, manifest.build_identifier,
                qualification_projection, registry_runtime_state,
                options.mark_non_uproperty_properties_as_transient);
            if (!projected.succeeded()) {
                return failure(
                    wire::ExitCode::software, "invalid_output",
                    "GORE_AS_QUALIFICATION_PROJECTION_FAILED", projected.detail, diagnostics);
            }
            response_cache = &qualification_projection;
        }
        std::vector<std::uint8_t> encoded;
        if (!cache_wire::encode(*response_cache, encoded, codec)) {
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_CACHE_ENCODE_FAILED",
                codec.field + " at " + std::to_string(codec.offset) + ": " + codec.detail);
        }
        const sha256_digest output_digest = sha256_bytes(encoded.data(), encoded.size());
        auto response = success(
            request, encoded.size(), output_digest, diagnostics,
            request.qualification_mode ? &qualification_evidence : nullptr);
        if (response.exit_code != wire::ExitCode::success) return response;
        if (!write_output_new(request.output_path, encoded, detail)) {
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_OUTPUT_CREATE_FAILED", detail);
        }
        return response;
    } catch (const std::exception& exception) {
        return failure(wire::ExitCode::software, "internal", "GORE_AS_INTERNAL", exception.what());
    } catch (...) {
        return failure(wire::ExitCode::software, "internal", "GORE_AS_INTERNAL", "unknown native compiler failure");
    }
}

} // namespace gore::as::standalone
