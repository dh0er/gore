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

struct compile_request {
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
    std::string output_path_utf8;
    std::wstring output_path;
};

struct compiler_diagnostic {
    std::string severity;
    std::string code;
    std::string message;
    std::string source;
    std::uint32_t line = 0U;
    std::uint32_t column = 0U;
};

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
    const std::vector<compiler_diagnostic>& diagnostics) {
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
    json += "]}\n";
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

bool parse_request(const std::string_view bytes, compile_request& output, std::string& detail) {
    json::value root;
    json::parse_error error;
    if (!json::parse(bytes, wire::kMaxJsonNestingDepth, root, error)) {
        detail = "request JSON offset " + std::to_string(error.offset) + ": " + error.detail;
        return false;
    }
    if (!json::require_object_keys(root, {"request_version", "operation", "profile", "inputs", "output"}, {}, detail)) return false;
    std::uint64_t version = 0U;
    std::string operation;
    if (!json::get_u64(root, "request_version", version, detail) || version != wire::kRequestProtocolVersion ||
        !json::get_string(root, "operation", operation, detail) || operation != "compile") {
        detail = "unsupported request version or operation";
        return false;
    }
    compile_request staged;
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
    const json::value* overlays = nullptr;
    if (!json::get_object(root, "inputs", inputs, detail) ||
        !json::require_object_keys(*inputs, {"base_cache", "binds_cache", "source_tree", "overlays"}, {}, detail) ||
        !json::get_object(*inputs, "base_cache", base, detail) || !parse_path_seal(*base, staged.base_cache, detail) ||
        !json::get_object(*inputs, "binds_cache", binds, detail) || !parse_path_seal(*binds, staged.binds_cache, detail) ||
        !json::get_object(*inputs, "source_tree", source_tree, detail) ||
        !json::require_object_keys(*source_tree, {"root", "files"}, {}, detail) ||
        !json::get_string(*source_tree, "root", staged.source_root_utf8, detail) ||
        !absolute_path(staged.source_root_utf8, staged.source_root, detail) ||
        !json::get_array(*source_tree, "files", files, detail) || files->elements.size() > wire::kMaxSourceFiles ||
        !json::get_array(*inputs, "overlays", overlays, detail) || overlays->elements.empty() ||
        overlays->elements.size() > wire::kMaxOverlayModules) return false;

    std::set<std::string> file_paths;
    std::uint64_t aggregate = 0U;
    for (const auto& item : files->elements) {
        source_file file;
        std::string file_digest;
        if (!json::require_object_keys(item, {"path", "byte_len", "sha256"}, {}, detail) ||
            !json::get_string(item, "path", file.relative_path, detail) || !safe_relative_path(file.relative_path) ||
            !json::get_u64(item, "byte_len", file.byte_len, detail) || file.byte_len > wire::kMaxSourceFileBytes ||
            !json::get_string(item, "sha256", file_digest, detail) || !parse_sha256_hex(file_digest, file.sha256) ||
            !file_paths.insert(file.relative_path).second) {
            if (detail.empty()) detail = "source tree contains an invalid or duplicate file";
            return false;
        }
        if (aggregate > wire::kMaxAggregateSourceBytes - file.byte_len) {
            detail = "source tree exceeds the aggregate byte limit";
            return false;
        }
        aggregate += file.byte_len;
        staged.source_files.push_back(std::move(file));
    }

    std::set<std::string> module_names;
    std::set<std::string> overlay_paths;
    for (const auto& item : overlays->elements) {
        overlay_module overlay;
        std::string operation_name;
        if (!json::require_object_keys(item, {"ordinal", "operation", "module_name", "relative_path"}, {}, detail) ||
            !json::get_u64(item, "ordinal", version, detail) || version != staged.overlays.size() ||
            !json::get_string(item, "operation", operation_name, detail) ||
            !json::get_string(item, "module_name", overlay.module_name, detail) || overlay.module_name.empty() ||
            overlay.module_name.size() > wire::kMaxModuleIdentityBytes ||
            !json::get_string(item, "relative_path", overlay.relative_path, detail) ||
            !safe_relative_path(overlay.relative_path) || file_paths.count(overlay.relative_path) != 1U ||
            !module_names.insert(overlay.module_name).second || !overlay_paths.insert(overlay.relative_path).second) {
            if (detail.empty()) detail = "overlay manifest is invalid or colliding";
            return false;
        }
        overlay.ordinal = static_cast<std::uint32_t>(version);
        if (operation_name == "add") overlay.operation = preprocessor_source::operation::add;
        else if (operation_name == "edit") overlay.operation = preprocessor_source::operation::edit;
        else { detail = "unsupported overlay operation"; return false; }
        staged.overlays.push_back(std::move(overlay));
    }

    const json::value* output_object = nullptr;
    if (!json::get_object(root, "output", output_object, detail) ||
        !json::require_object_keys(*output_object, {"cache_path"}, {}, detail) ||
        !json::get_string(*output_object, "cache_path", staged.output_path_utf8, detail) ||
        !absolute_path(staged.output_path_utf8, staged.output_path, detail)) return false;
    output = std::move(staged);
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

void message_callback(const asSMessageInfo* message, void* parameter) {
    auto& diagnostics = *static_cast<std::vector<compiler_diagnostic>*>(parameter);
    if (message == nullptr || diagnostics.size() >= wire::kMaxDiagnostics) return;
    const char* severity = message->type == asMSGTYPE_ERROR ? "error" :
        (message->type == asMSGTYPE_WARNING ? "warning" : "info");
    diagnostics.push_back({
        severity,
        message->type == asMSGTYPE_ERROR ? "GORE_AS_COMPILER_ERROR" :
            (message->type == asMSGTYPE_WARNING ? "GORE_AS_COMPILER_WARNING" : "GORE_AS_COMPILER_INFO"),
        message->message == nullptr ? "" : message->message,
        message->section == nullptr ? "" : message->section,
        message->row < 0 ? 0U : static_cast<std::uint32_t>(message->row),
        message->col < 0 ? 0U : static_cast<std::uint32_t>(message->col),
    });
}

} // namespace

sidecar_compile_result compile_sidecar_request(const std::wstring_view request_path) noexcept {
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
        if (!parse_request(request_text, request, detail)) {
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
        if (!parse_compiler_profile_manifest(manifest_text, manifest, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_PROFILE_INVALID", detail);
        }
        if (manifest.profile_sha256 != request.profile_sha256 ||
            manifest.steam_build_id != request.steam_build_id || manifest.depot_id != request.depot_id ||
            manifest.depot_manifest_gid != request.depot_manifest_gid ||
            manifest.required_probe_suite_version != request.required_probe_suite_version) {
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
        if (!parse_frontend_profile_payloads(
                text_blob(manifest.preprocessor_config), text_blob(manifest.class_generator_config),
                text_blob(manifest.compiler_options), preprocessor, options, detail)) {
            return failure(wire::ExitCode::unavailable, "engine_unavailable", "GORE_AS_FRONTEND_PROFILE_INVALID", detail);
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

        std::map<std::string, std::string> source_contents;
        for (const auto& file : request.source_files) {
            const std::wstring path = joined(request.source_root, file.relative_path, detail);
            std::vector<std::uint8_t> bytes;
            if (path.empty() || !read_file(path, wire::kMaxSourceFileBytes, bytes, detail, &file.byte_len, &file.sha256)) {
                return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_SOURCE_FILE_INVALID", detail);
            }
            std::string code(reinterpret_cast<const char*>(bytes.data()), bytes.size());
            if (code.find('\0') != std::string::npos || !is_valid_utf8(code)) {
                return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_SOURCE_ENCODING_INVALID",
                    "source file must be canonical UTF-8 without NUL bytes");
            }
            source_contents.emplace(file.relative_path, std::move(code));
        }
        std::vector<preprocessor_source> sources;
        sources.reserve(request.overlays.size());
        for (const auto& overlay : request.overlays) {
            preprocessor_source source;
            source.relative_path = overlay.relative_path;
            source.absolute_path = request.source_root_utf8 + "/" + overlay.relative_path;
            source.code = source_contents.at(overlay.relative_path);
            source.overlay_operation = overlay.operation;
            source.module_name = overlay.module_name;
            sources.push_back(std::move(source));
        }
        preprocessor.static_names.reserve(base.static_names.size());
        for (const auto& name : base.static_names) preprocessor.static_names.push_back(name.bytes);
        const auto preprocessing = preprocess_lexical_module_graph(preprocessor, sources, base_modules(base));
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
        std::vector<asIScriptModule*> modules;
        const auto compiled = cache_wire::compile_mixed_cache_checkpoint(
            *engine, base, preprocessor, preprocessing, &registry_runtime_state, frontend_runtime, modules);
        if (!compiled.succeeded()) {
            return failure(wire::ExitCode::data_error, "rejected", "GORE_AS_COMPILE_REJECTED",
                compiled.detail, diagnostics);
        }

        sha256 guid_hash;
        constexpr char guid_domain[] = "gore-as-output-guid-v1\0";
        guid_hash.update(guid_domain, sizeof(guid_domain) - 1U);
        guid_hash.update(request.profile_sha256.data(), request.profile_sha256.size());
        guid_hash.update(request.base_cache.sha256.data(), request.base_cache.sha256.size());
        for (const auto& overlay : request.overlays) {
            guid_hash.update(overlay.module_name);
            guid_hash.update(std::string_view("\0", 1U));
            guid_hash.update(overlay.relative_path);
            guid_hash.update(std::string_view("\0", 1U));
            const auto& file = *std::find_if(request.source_files.begin(), request.source_files.end(),
                [&](const source_file& candidate) { return candidate.relative_path == overlay.relative_path; });
            guid_hash.update(file.sha256.data(), file.sha256.size());
        }
        const auto guid_digest = guid_hash.finish();
        std::array<std::uint8_t, 16U> guid{};
        std::copy_n(guid_digest.begin(), guid.size(), guid.begin());
        cache_wire::cache generated;
        const auto exported = cache_wire::export_mixed_graph_checkpoint(
            base, preprocessing, modules, guid, manifest.build_identifier, generated);
        if (!exported.succeeded()) {
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_CACHE_EXPORT_FAILED", exported.detail);
        }
        std::vector<std::uint8_t> encoded;
        if (!cache_wire::encode(generated, encoded, codec)) {
            return failure(wire::ExitCode::software, "invalid_output", "GORE_AS_CACHE_ENCODE_FAILED",
                codec.field + " at " + std::to_string(codec.offset) + ": " + codec.detail);
        }
        const sha256_digest output_digest = sha256_bytes(encoded.data(), encoded.size());
        auto response = success(request, encoded.size(), output_digest, diagnostics);
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
