#include "materializer.hpp"

#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cwchar>
#include <limits>
#include <new>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace gore_as_capture::v1::offline {
namespace {

class Handle final {
 public:
  explicit Handle(const HANDLE value = INVALID_HANDLE_VALUE) noexcept : value_(value) {}
  ~Handle() { (void)close(); }
  Handle(Handle&& other) noexcept : value_(other.release()) {}
  Handle& operator=(Handle&& other) noexcept {
    if (this != &other) {
      (void)close();
      value_ = other.release();
    }
    return *this;
  }
  Handle(const Handle&) = delete;
  Handle& operator=(const Handle&) = delete;
  [[nodiscard]] bool valid() const noexcept {
    return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
  }
  [[nodiscard]] HANDLE get() const noexcept { return value_; }
  [[nodiscard]] HANDLE release() noexcept {
    const HANDLE value = value_;
    value_ = INVALID_HANDLE_VALUE;
    return value;
  }
  [[nodiscard]] bool close() noexcept {
    if (valid()) {
      return CloseHandle(release()) != FALSE;
    }
    return true;
  }

 private:
  HANDLE value_;
};

struct ParsedSummary final {
  std::uint64_t record_count{};
  Digest sealed_stream_sha256{};
  GuidBytes capture_id{};
  std::array<std::uint64_t, 11> kind_counts{};
  std::uint32_t compiler_build_flags{};
};

class Cursor final {
 public:
  explicit Cursor(const std::span<const std::byte> bytes) noexcept : bytes_(bytes) {}

  [[nodiscard]] std::optional<std::span<const std::byte>> take(const std::size_t count) noexcept {
    if (count > bytes_.size() - offset_) {
      return std::nullopt;
    }
    const auto result = bytes_.subspan(offset_, count);
    offset_ += count;
    return result;
  }

  [[nodiscard]] std::optional<std::uint16_t> u16() noexcept {
    const auto bytes = take(2);
    if (!bytes.has_value()) {
      return std::nullopt;
    }
    const auto low = static_cast<std::uint16_t>(std::to_integer<std::uint8_t>((*bytes)[0]));
    const auto high = static_cast<std::uint16_t>(std::to_integer<std::uint8_t>((*bytes)[1]));
    return static_cast<std::uint16_t>(low | static_cast<std::uint16_t>(high << 8u));
  }

  [[nodiscard]] std::optional<std::uint32_t> u32() noexcept {
    const auto bytes = take(4);
    if (!bytes.has_value()) {
      return std::nullopt;
    }
    std::uint32_t value = 0;
    for (unsigned index = 0; index < 4; ++index) {
      value |= static_cast<std::uint32_t>(std::to_integer<std::uint8_t>((*bytes)[index]))
               << (index * 8u);
    }
    return value;
  }

  [[nodiscard]] std::optional<std::uint64_t> u64() noexcept {
    const auto bytes = take(8);
    if (!bytes.has_value()) {
      return std::nullopt;
    }
    std::uint64_t value = 0;
    for (unsigned index = 0; index < 8; ++index) {
      value |= static_cast<std::uint64_t>(std::to_integer<std::uint8_t>((*bytes)[index]))
               << (index * 8u);
    }
    return value;
  }

  [[nodiscard]] std::size_t offset() const noexcept { return offset_; }
  [[nodiscard]] bool empty() const noexcept { return offset_ == bytes_.size(); }

 private:
  std::span<const std::byte> bytes_;
  std::size_t offset_{};
};

bool regular_no_reparse(const HANDLE file) noexcept {
  FILE_ATTRIBUTE_TAG_INFO attributes{};
  return GetFileInformationByHandleEx(
             file, FileAttributeTagInfo, &attributes, sizeof(attributes)) != FALSE &&
         (attributes.FileAttributes &
          (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) == 0;
}

bool directory_no_reparse(const HANDLE directory) noexcept {
  FILE_ATTRIBUTE_TAG_INFO attributes{};
  return GetFileInformationByHandleEx(
             directory, FileAttributeTagInfo, &attributes, sizeof(attributes)) != FALSE &&
         (attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 &&
         (attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) == 0;
}

std::optional<std::filesystem::path> final_path(const HANDLE handle) {
  const DWORD required = GetFinalPathNameByHandleW(handle, nullptr, 0, 0);
  if (required == 0 || required >= 32768) {
    return std::nullopt;
  }
  std::vector<wchar_t> buffer(static_cast<std::size_t>(required) + 1u, L'\0');
  const DWORD length = GetFinalPathNameByHandleW(
      handle, buffer.data(), static_cast<DWORD>(buffer.size()), 0);
  if (length == 0 || static_cast<std::size_t>(length) >= buffer.size()) {
    return std::nullopt;
  }
  return std::filesystem::path(std::wstring_view(buffer.data(), length)).lexically_normal();
}

bool equal_path_case_insensitive(
    const std::filesystem::path& left,
    const std::filesystem::path& right) noexcept {
  auto left_it = left.begin();
  auto right_it = right.begin();
  while (left_it != left.end() && right_it != right.end()) {
    if (_wcsicmp(left_it->c_str(), right_it->c_str()) != 0) {
      return false;
    }
    ++left_it;
    ++right_it;
  }
  return left_it == left.end() && right_it == right.end();
}

bool write_all(const HANDLE file, std::span<const std::byte> bytes) noexcept {
  while (!bytes.empty()) {
    const DWORD request =
        static_cast<DWORD>(std::min<std::size_t>(bytes.size(), 1u << 30u));
    DWORD written = 0;
    if (WriteFile(file, bytes.data(), request, &written, nullptr) == FALSE || written != request) {
      return false;
    }
    bytes = bytes.subspan(written);
  }
  return true;
}

bool read_all(const HANDLE file, std::span<std::byte> bytes) noexcept {
  while (!bytes.empty()) {
    const DWORD request =
        static_cast<DWORD>(std::min<std::size_t>(bytes.size(), 1u << 30u));
    DWORD read = 0;
    if (ReadFile(file, bytes.data(), request, &read, nullptr) == FALSE || read != request) {
      return false;
    }
    bytes = bytes.subspan(read);
  }
  return true;
}

bool sha256(
    const std::span<const std::byte> prefix,
    const std::span<const std::byte> bytes,
    Digest& digest) noexcept {
  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  std::vector<UCHAR> object;
  bool ok = false;
  do {
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0) {
      break;
    }
    DWORD object_bytes = 0;
    DWORD returned = 0;
    if (BCryptGetProperty(
            algorithm,
            BCRYPT_OBJECT_LENGTH,
            reinterpret_cast<PUCHAR>(&object_bytes),
            sizeof(object_bytes),
            &returned,
            0) < 0) {
      break;
    }
    object.resize(object_bytes);
    if (BCryptCreateHash(algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
      break;
    }
    const auto append = [hash](const std::span<const std::byte> data) {
      std::size_t offset = 0;
      while (offset < data.size()) {
        const ULONG size = static_cast<ULONG>(
            std::min<std::size_t>(data.size() - offset, std::numeric_limits<ULONG>::max()));
        if (BCryptHashData(
                hash,
                reinterpret_cast<PUCHAR>(const_cast<std::byte*>(data.data() + offset)),
                size,
                0) < 0) {
          return false;
        }
        offset += size;
      }
      return true;
    };
    if (!append(prefix) || !append(bytes) ||
        BCryptFinishHash(
            hash,
            reinterpret_cast<PUCHAR>(digest.data()),
            static_cast<ULONG>(digest.size()),
            0) < 0) {
      break;
    }
    ok = true;
  } while (false);
  if (hash != nullptr) {
    BCryptDestroyHash(hash);
  }
  if (algorithm != nullptr) {
    BCryptCloseAlgorithmProvider(algorithm, 0);
  }
  return ok;
}

bool all_zero(const std::span<const std::byte> bytes) noexcept {
  return std::all_of(bytes.begin(), bytes.end(), [](const std::byte value) {
    return value == std::byte{0};
  });
}

bool equal_bytes(
    const std::span<const std::byte> left,
    const std::span<const std::byte> right) noexcept {
  return left.size() == right.size() && std::equal(left.begin(), left.end(), right.begin());
}

bool json_shape(const std::span<const std::byte> payload) noexcept {
  return !payload.empty() && payload.front() == std::byte{'{'} &&
         payload.back() == std::byte{'}'} &&
         std::find(payload.begin(), payload.end(), std::byte{0}) == payload.end();
}

bool has_named_stream(const std::filesystem::path& path) noexcept {
  return path.filename().native().find(L':') != std::wstring::npos;
}

MaterializeError parse_capture(
    const std::span<const std::byte> bytes,
    ParsedSummary& summary) noexcept {
  if (bytes.size() < kHeaderBytes + kFooterBytes) {
    return MaterializeError::malformed_capture;
  }
  const std::size_t footer_offset = bytes.size() - kFooterBytes;
  Cursor header(bytes.first(kHeaderBytes));
  const auto magic = header.take(kCaptureMagic.size());
  const auto schema = header.u16();
  const auto header_bytes = header.u16();
  const auto header_reserved = header.u32();
  const auto app_id = header.u32();
  const auto build_id = header.u64();
  const auto angelscript_version = header.u32();
  const auto executable_bytes = header.u64();
  const auto executable_sha = header.take(kExecutableSha256.size());
  const auto codeview_guid = header.take(kCodeViewGuidRsds.size());
  const auto codeview_age = header.u32();
  const auto capture_id = header.take(summary.capture_id.size());
  const auto final_reserved = header.u32();
  if (!magic.has_value() || !schema.has_value() || !header_bytes.has_value() ||
      !header_reserved.has_value() || !app_id.has_value() || !build_id.has_value() ||
      !angelscript_version.has_value() || !executable_bytes.has_value() ||
      !executable_sha.has_value() || !codeview_guid.has_value() || !codeview_age.has_value() ||
      !capture_id.has_value() || !final_reserved.has_value() || !header.empty()) {
    return MaterializeError::malformed_capture;
  }
  if (!equal_bytes(*magic, kCaptureMagic) || *schema != kSchemaVersion ||
      *header_bytes != kHeaderBytes || *header_reserved != 0 || *final_reserved != 0 ||
      *app_id != kSteamAppId || *build_id != kSteamBuildId ||
      *angelscript_version != kAngelScriptVersion || *executable_bytes != kExecutableBytes ||
      !equal_bytes(*executable_sha, kExecutableSha256) ||
      !equal_bytes(*codeview_guid, kCodeViewGuidRsds) || *codeview_age != kCodeViewAge) {
    return MaterializeError::target_mismatch;
  }
  if (all_zero(*capture_id)) {
    return MaterializeError::malformed_capture;
  }
  std::memcpy(summary.capture_id.data(), capture_id->data(), summary.capture_id.size());

  Cursor footer(bytes.subspan(footer_offset));
  const auto footer_magic = footer.take(kFooterMagic.size());
  const auto footer_records = footer.u64();
  const auto stream_bytes = footer.u64();
  const auto stored_digest = footer.take(summary.sealed_stream_sha256.size());
  const auto footer_schema = footer.u32();
  const auto footer_reserved = footer.u32();
  if (!footer_magic.has_value() || !footer_records.has_value() || !stream_bytes.has_value() ||
      !stored_digest.has_value() || !footer_schema.has_value() || !footer_reserved.has_value() ||
      !footer.empty() || !equal_bytes(*footer_magic, kFooterMagic) ||
      *stream_bytes != footer_offset || *footer_schema != kSchemaVersion ||
      *footer_reserved != 0 || *footer_records > kMaxRecords) {
    return MaterializeError::malformed_capture;
  }
  Digest computed{};
  if (!sha256(kHashDomain, bytes.first(footer_offset), computed)) {
    return MaterializeError::crypto_error;
  }
  if (!equal_bytes(*stored_digest, computed)) {
    return MaterializeError::digest_mismatch;
  }
  summary.sealed_stream_sha256 = computed;

  Cursor records(bytes.subspan(kHeaderBytes, footer_offset - kHeaderBytes));
  bool property_seen = false;
  bool bind_active = false;
  bool bind_seen = false;
  bool support_seen = false;
  bool final_seen = false;
  bool build_seen = false;
  std::uint32_t config_mask = 0;
  std::uint32_t boundary_count = 0;
  std::uint32_t pointer_count = 0;
  std::uint32_t next_callback = 0;
  std::uint32_t active_callback = 0;
  std::int32_t active_bind_order = 0;
  std::uint32_t active_pointer_token = 0;
  std::optional<Digest> frontend_config_digest;
  std::uint64_t ordinal = 0;
  while (!records.empty()) {
    const auto kind = records.u16();
    const auto kind_version = records.u16();
    const auto flags = records.u32();
    const auto payload_bytes = records.u32();
    const auto reserved = records.u32();
    const auto record_ordinal = records.u64();
    if (!kind.has_value() || !kind_version.has_value() || !flags.has_value() ||
        !payload_bytes.has_value() || !reserved.has_value() || !record_ordinal.has_value() ||
        *kind < 1 || *kind > 10 || *kind_version != kSchemaVersion || *flags != 0 ||
        *reserved != 0 || *record_ordinal != ordinal || *payload_bytes > kMaxPayloadBytes) {
    return MaterializeError::malformed_capture;
    }
    const auto payload = records.take(*payload_bytes);
    if (!payload.has_value()) {
    return MaterializeError::malformed_capture;
    }
    ++summary.kind_counts[*kind];
    switch (static_cast<RecordKind>(*kind)) {
      case RecordKind::engine_property:
        if (payload->size() != 24 || bind_active || bind_seen || support_seen) {
    return MaterializeError::malformed_capture;
        }
        {
          Cursor property(*payload);
          const auto property_id = property.u32();
          const auto reserved0 = property.u32();
          const auto value = property.u64();
          const auto observation_rva = property.u32();
          const auto reserved1 = property.u32();
          if (!property_id.has_value() || !reserved0.has_value() || !value.has_value() ||
              !observation_rva.has_value() || !reserved1.has_value() || !property.empty() ||
              *property_id == 0 || *property_id > 34 || *reserved0 != 0 || *reserved1 != 0 ||
              *observation_rva != kRvaSetEngineProperty) {
    return MaterializeError::malformed_capture;
          }
        }
        property_seen = true;
        break;
      case RecordKind::pointer_token:
        if (payload->size() != 12 || final_seen || build_seen) {
    return MaterializeError::malformed_capture;
        }
        {
          Cursor pointer(*payload);
          const auto token = pointer.u32();
          const auto image_rva = pointer.u32();
          const auto pointer_reserved = pointer.u32();
          if (!token.has_value() || !image_rva.has_value() || !pointer_reserved.has_value() ||
              !pointer.empty() || *token != pointer_count || *image_rva == 0 ||
              *image_rva >= kPeSizeOfImage || *pointer_reserved != 0) {
    return MaterializeError::malformed_capture;
          }
          ++pointer_count;
        }
        break;
      case RecordKind::bind_callback: {
        if (payload->size() != 88 || !property_seen || support_seen) {
    return MaterializeError::malformed_capture;
        }
        Cursor bind(*payload);
        const auto callback = bind.u32();
        const auto phase = bind.u32();
        const auto bind_order = bind.u32();
        const auto pointer_token = bind.u32();
        const auto observation_rva = bind.u32();
        const auto bind_reserved = bind.u32();
        const auto counts = bind.take(8 * sizeof(std::uint32_t));
        const auto registry_digest = bind.take(Digest{}.size());
        if (!callback.has_value() || !phase.has_value() || !bind_order.has_value() ||
            !pointer_token.has_value() || !observation_rva.has_value() ||
            !bind_reserved.has_value() || !counts.has_value() || !registry_digest.has_value() ||
            !bind.empty() || *bind_reserved != 0 || *pointer_token >= pointer_count ||
            all_zero(*registry_digest)) {
    return MaterializeError::malformed_capture;
        }
        if (*phase == 1 && !bind_active && *callback == next_callback &&
            *observation_rva == kRvaBindCallbackCall) {
          bind_active = true;
          active_callback = *callback;
          active_bind_order = static_cast<std::int32_t>(*bind_order);
          active_pointer_token = *pointer_token;
        } else if (*phase == 2 && bind_active) {
          if (*callback != active_callback ||
              static_cast<std::int32_t>(*bind_order) != active_bind_order ||
              *pointer_token != active_pointer_token ||
              *observation_rva != kRvaBindCallbackReturn) {
    return MaterializeError::malformed_capture;
          }
          bind_active = false;
          bind_seen = true;
          ++next_callback;
        } else {
    return MaterializeError::malformed_capture;
        }
        break;
      }
      case RecordKind::registry_delta_json:
      case RecordKind::post_bind_mutation_json:
        if (!bind_active || !json_shape(*payload)) {
    return MaterializeError::malformed_capture;
        }
        break;
      case RecordKind::registry_support_json:
        if (bind_active || !bind_seen || support_seen || !json_shape(*payload)) {
    return MaterializeError::malformed_capture;
        }
        support_seen = true;
        break;
      case RecordKind::final_post_bind_state_json:
        if (bind_active || !support_seen || build_seen || !json_shape(*payload)) {
    return MaterializeError::malformed_capture;
        }
        final_seen = true;
        break;
      case RecordKind::build_jit:
        if (payload->size() != 48 || bind_active || !final_seen || build_seen) {
    return MaterializeError::malformed_capture;
        }
        {
          Cursor build(*payload);
          const auto identifier = build.u32();
          const auto build_flags = build.u32();
          const auto precompiled_guid = build.take(kPrecompiledGuid.size());
          const auto compiled_jit_guid = build.take(GuidBytes{}.size());
          const auto build_rva = build.u32();
          const auto jit_rva = build.u32();
          if (!identifier.has_value() || !build_flags.has_value() ||
              !precompiled_guid.has_value() ||
              !compiled_jit_guid.has_value() || !build_rva.has_value() || !jit_rva.has_value() ||
              !build.empty() || *identifier != kBuildIdentifier ||
              (*build_flags & ~0xffu) != 0 || (*build_flags & 0xf0u) != 0x20u ||
              (*build_flags & 0x08u) == 0 ||
              (*build_flags & 0x04u) != 0 ||
              !equal_bytes(*precompiled_guid, kPrecompiledGuid) ||
              *build_rva != kRvaGetBuildIdentifier || *jit_rva != kRvaGetStaticJitInfo ||
              (((*build_flags & 0x01u) == 0) &&
               (((*build_flags & 0x02u) != 0) || !all_zero(*compiled_jit_guid))) ||
              (((*build_flags & 0x01u) != 0) &&
               (((*build_flags & 0x02u) == 0) ||
                !equal_bytes(*compiled_jit_guid, *precompiled_guid)))) {
    return MaterializeError::malformed_capture;
          }
          summary.compiler_build_flags = *build_flags;
        }
        build_seen = true;
        break;
      case RecordKind::frontend_config_json: {
        if (!build_seen || boundary_count != 0 || payload->size() <= 4) {
    return MaterializeError::malformed_capture;
        }
        Cursor config(*payload);
        const auto config_kind = config.u32();
        const auto json = config.take(payload->size() - 4);
        if (!config_kind.has_value() || !json.has_value() || *config_kind < 1 ||
            *config_kind > 3 || (config_mask & (1u << *config_kind)) != 0 ||
            !json_shape(*json)) {
    return MaterializeError::malformed_capture;
        }
        config_mask |= 1u << *config_kind;
        break;
      }
      case RecordKind::frontend_boundary: {
        if (payload->size() != 112 || config_mask != 0x0eu || boundary_count >= 3) {
    return MaterializeError::malformed_capture;
        }
        Cursor boundary(*payload);
        const auto boundary_kind = boundary.u32();
        const auto rva = boundary.u32();
        const auto module_count = boundary.u32();
        const auto result_code = boundary.u32();
        const auto config_digest = boundary.take(Digest{}.size());
        const auto input_digest = boundary.take(Digest{}.size());
        const auto output_digest = boundary.take(Digest{}.size());
        const bool expected = boundary_kind.has_value() && rva.has_value() &&
                              module_count.has_value() && result_code.has_value() &&
                              config_digest.has_value() && input_digest.has_value() &&
                              output_digest.has_value() && boundary.empty() &&
                              !all_zero(*config_digest) &&
                              ((boundary_count == 0 && *boundary_kind == 1 &&
                                *rva == kRvaInitialCompileEnter && *module_count == 0 &&
                                all_zero(*output_digest)) ||
                               (boundary_count == 1 &&
                                ((*boundary_kind == 2 &&
                                  *rva == kRvaPrecompiledDescriptorsRequested &&
                                  *module_count != 0 && !all_zero(*input_digest) &&
                                  !all_zero(*output_digest)) ||
                                 (*boundary_kind == 3 &&
                                  *rva == kRvaPreprocessorConstructed && *module_count == 0 &&
                                  all_zero(*input_digest) && all_zero(*output_digest)))) ||
                               (boundary_count == 2 && *boundary_kind == 4 &&
                                *rva == kRvaInitialCompileReturn && *module_count != 0 &&
                                *result_code == 0 && !all_zero(*output_digest)));
        if (!expected ||
            (frontend_config_digest.has_value() &&
             !equal_bytes(*config_digest, *frontend_config_digest))) {
    return MaterializeError::malformed_capture;
        }
        if (!frontend_config_digest.has_value()) {
          Digest copied{};
          std::memcpy(copied.data(), config_digest->data(), copied.size());
          frontend_config_digest = copied;
        }
        ++boundary_count;
        break;
      }
    }
    ++ordinal;
  }
  if (ordinal != *footer_records) {
    return MaterializeError::malformed_capture;
  }
  if (!property_seen || bind_active || !bind_seen ||
      summary.kind_counts[static_cast<std::size_t>(RecordKind::registry_delta_json)] == 0 ||
      !support_seen || !final_seen || !build_seen || config_mask != 0x0eu ||
      boundary_count != 3) {
    return MaterializeError::incomplete_capture;
  }
  summary.record_count = ordinal;
  return MaterializeError::ok;
}

std::string hex(const std::span<const std::byte> bytes) {
  constexpr std::array<char, 16> digits{
      '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'};
  std::string output;
  output.reserve(bytes.size() * 2);
  for (const std::byte byte : bytes) {
    const auto value = std::to_integer<std::uint8_t>(byte);
    output.push_back(digits[value >> 4u]);
    output.push_back(digits[value & 0x0fu]);
  }
  return output;
}

std::string summary_json(const ParsedSummary& summary) {
  std::string output;
  output.reserve(1024);
  output += "{\n  \"schema\": \"gore.as.capture-wire-materialization\",\n";
  output += "  \"schema_version\": 1,\n";
  output += "  \"scope\": \"wire_only_not_a_qualified_compiler_profile\",\n";
  output += "  \"steam_app_id\": " + std::to_string(kSteamAppId) + ",\n";
  output += "  \"steam_build_id\": " + std::to_string(kSteamBuildId) + ",\n";
  output += "  \"capture_id\": \"" + hex(summary.capture_id) + "\",\n";
  output += "  \"sealed_stream_sha256\": \"" + hex(summary.sealed_stream_sha256) + "\",\n";
  output += "  \"record_count\": " + std::to_string(summary.record_count) + ",\n";
  output += "  \"compiler_build_flags\": {\n";
  output += "    \"as_reference_debugging\": " +
            std::string((summary.compiler_build_flags & 0x10u) != 0 ? "true" : "false") +
            ",\n";
  output += "    \"fork_opcode_table_201_212_present\": " +
            std::string((summary.compiler_build_flags & 0x20u) != 0 ? "true" : "false") +
            ",\n";
  output += "    \"reference_debug_opcodes_emittable\": " +
            std::string((summary.compiler_build_flags & 0x40u) != 0 ? "true" : "false") +
            ",\n";
  output += "    \"resolve_object_ptr_callback_registered\": " +
            std::string((summary.compiler_build_flags & 0x80u) != 0 ? "true" : "false") +
            "\n  },\n";
  output += "  \"record_kinds\": {\n";
  constexpr std::array<std::string_view, 10> names{
      "engine_property",
      "pointer_token",
      "bind_callback",
      "registry_delta_json",
      "post_bind_mutation_json",
      "final_post_bind_state_json",
      "build_jit",
      "frontend_boundary",
      "frontend_config_json",
      "registry_support_json",
  };
  for (std::size_t index = 0; index < names.size(); ++index) {
    output += "    \"";
    output += names[index];
    output += "\": " + std::to_string(summary.kind_counts[index + 1]);
    output += index + 1 == names.size() ? "\n" : ",\n";
  }
  output += "  }\n}\n";
  return output;
}

MaterializeError reject_output(
    Handle& output,
    const MaterializeError rejection) noexcept {
  FILE_DISPOSITION_INFO disposition{};
  disposition.DeleteFile = TRUE;
  const bool marked = SetFileInformationByHandle(
                          output.get(), FileDispositionInfo, &disposition, sizeof(disposition)) !=
                      FALSE;
  const bool closed = output.close();
  return marked && closed ? rejection
                          : MaterializeError::output_recovery_required;
}

}  // namespace

MaterializeResult materialize_capture_summary_v1(
    const std::filesystem::path& capture_path,
    const std::filesystem::path& summary_path) noexcept {
  MaterializeResult result;
  try {
    if (capture_path.empty() || summary_path.empty() || has_named_stream(capture_path) ||
        has_named_stream(summary_path)) {
      result.error = MaterializeError::invalid_argument;
      return result;
    }
    Handle input(CreateFileW(
        capture_path.c_str(),
        GENERIC_READ,
        FILE_SHARE_READ,
        nullptr,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_SEQUENTIAL_SCAN,
        nullptr));
    if (!input.valid()) {
      result.error = MaterializeError::input_io;
      return result;
    }
    if (!regular_no_reparse(input.get())) {
      result.error = MaterializeError::input_reparse;
      return result;
    }
    LARGE_INTEGER input_size{};
    if (GetFileSizeEx(input.get(), &input_size) == FALSE || input_size.QuadPart < 0) {
      result.error = MaterializeError::input_io;
      return result;
    }
    if (static_cast<std::uint64_t>(input_size.QuadPart) > kMaxCaptureBytes) {
      result.error = MaterializeError::input_too_large;
      return result;
    }
    std::vector<std::byte> bytes(static_cast<std::size_t>(input_size.QuadPart));
    if (!read_all(input.get(), bytes)) {
      result.error = MaterializeError::input_io;
      return result;
    }
    ParsedSummary parsed;
    result.error = parse_capture(bytes, parsed);
    if (result.error != MaterializeError::ok) {
      return result;
    }
    result.record_count = parsed.record_count;
    result.sealed_stream_sha256 = parsed.sealed_stream_sha256;
    const std::string json = summary_json(parsed);

    const auto absolute_summary = std::filesystem::absolute(summary_path).lexically_normal();
    if (absolute_summary.empty() || absolute_summary.parent_path().empty() ||
        absolute_summary.filename().empty()) {
      result.error = MaterializeError::output_unsafe;
      return result;
    }
    Handle output_parent(CreateFileW(
        absolute_summary.parent_path().c_str(),
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        nullptr,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        nullptr));
    const auto pinned_parent_path =
        output_parent.valid() ? final_path(output_parent.get()) : std::nullopt;
    if (!output_parent.valid() || !directory_no_reparse(output_parent.get()) ||
        !pinned_parent_path.has_value()) {
      result.error = MaterializeError::output_unsafe;
      return result;
    }

    Handle output(CreateFileW(
        absolute_summary.c_str(),
        GENERIC_READ | GENERIC_WRITE | DELETE,
        0,
        nullptr,
        CREATE_NEW,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
        nullptr));
    if (!output.valid()) {
      const DWORD error = GetLastError();
      result.error = error == ERROR_FILE_EXISTS || error == ERROR_ALREADY_EXISTS
                         ? MaterializeError::output_exists
                         : MaterializeError::output_io;
      return result;
    }
    const auto output_final = final_path(output.get());
    if (!regular_no_reparse(output.get()) || !output_final.has_value() ||
        !equal_path_case_insensitive(output_final->parent_path(), *pinned_parent_path)) {
      result.error = reject_output(output, MaterializeError::output_unsafe);
      return result;
    }
    const auto json_bytes = std::as_bytes(std::span(json));
    if (!write_all(output.get(), json_bytes) || FlushFileBuffers(output.get()) == FALSE) {
      result.error = reject_output(output, MaterializeError::output_io);
      return result;
    }
    if (!output.close()) {
      result.error = MaterializeError::output_recovery_required;
      return result;
    }
    result.error = MaterializeError::ok;
    return result;
  } catch (const std::bad_alloc&) {
    result.error = MaterializeError::input_too_large;
    return result;
  } catch (...) {
    result.error = MaterializeError::invalid_argument;
    return result;
  }
}

const char* materialize_error_name(const MaterializeError error) noexcept {
  switch (error) {
    case MaterializeError::ok:
      return "ok";
    case MaterializeError::invalid_argument:
      return "invalid_argument";
    case MaterializeError::input_io:
      return "input_io";
    case MaterializeError::input_reparse:
      return "input_reparse";
    case MaterializeError::input_too_large:
      return "input_too_large";
    case MaterializeError::malformed_capture:
      return "malformed_capture";
    case MaterializeError::target_mismatch:
      return "target_mismatch";
    case MaterializeError::digest_mismatch:
      return "digest_mismatch";
    case MaterializeError::incomplete_capture:
      return "incomplete_capture";
    case MaterializeError::output_exists:
      return "output_exists";
    case MaterializeError::output_unsafe:
      return "output_unsafe";
    case MaterializeError::output_io:
      return "output_io";
    case MaterializeError::output_recovery_required:
      return "output_recovery_required";
    case MaterializeError::crypto_error:
      return "crypto_error";
  }
  return "unknown";
}

}  // namespace gore_as_capture::v1::offline
