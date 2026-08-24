#include "gore_as_capture/session.hpp"
#include "path_safety.hpp"

#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <cwchar>
#include <limits>
#include <new>
#include <optional>
#include <utility>
#include <vector>

namespace gore_as_capture::v1 {
namespace {

constexpr std::uint32_t kJsonPayloadLimit = 256u * 1024u * 1024u;

void put_u16(std::vector<std::byte>& out, const std::uint16_t value) {
  out.push_back(static_cast<std::byte>(value & 0xffu));
  out.push_back(static_cast<std::byte>((value >> 8u) & 0xffu));
}

void put_u32(std::vector<std::byte>& out, const std::uint32_t value) {
  for (unsigned shift = 0; shift < 32; shift += 8) {
    out.push_back(static_cast<std::byte>((value >> shift) & 0xffu));
  }
}

void put_i32(std::vector<std::byte>& out, const std::int32_t value) {
  put_u32(out, static_cast<std::uint32_t>(value));
}

void put_u64(std::vector<std::byte>& out, const std::uint64_t value) {
  for (unsigned shift = 0; shift < 64; shift += 8) {
    out.push_back(static_cast<std::byte>((value >> shift) & 0xffu));
  }
}

template <std::size_t N>
void put_array(std::vector<std::byte>& out, const std::array<std::byte, N>& value) {
  out.insert(out.end(), value.begin(), value.end());
}

bool write_all(const HANDLE file, std::span<const std::byte> bytes) noexcept {
  while (!bytes.empty()) {
    const auto amount = static_cast<DWORD>(std::min<std::size_t>(bytes.size(), 1u << 30));
    DWORD written = 0;
    if (!WriteFile(file, bytes.data(), amount, &written, nullptr) || written != amount) {
      return false;
    }
    bytes = bytes.subspan(written);
  }
  return true;
}

bool hash_handle_prefix(
    const HANDLE file,
    const std::uint64_t byte_count,
    std::span<const std::byte> domain,
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
    DWORD result_bytes = 0;
    if (BCryptGetProperty(
            algorithm,
            BCRYPT_OBJECT_LENGTH,
            reinterpret_cast<PUCHAR>(&object_bytes),
            sizeof(object_bytes),
            &result_bytes,
            0) < 0) {
      break;
    }
    object.resize(object_bytes);
    if (BCryptCreateHash(algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
      break;
    }
    if (!domain.empty() &&
        BCryptHashData(
            hash,
            reinterpret_cast<PUCHAR>(const_cast<std::byte*>(domain.data())),
            static_cast<ULONG>(domain.size()),
            0) < 0) {
      break;
    }
    LARGE_INTEGER zero{};
    if (!SetFilePointerEx(file, zero, nullptr, FILE_BEGIN)) {
      break;
    }
    // Keep capture sealing independent of the host thread's stack reservation. A 1 MiB local
    // array exhausts the default Windows stack once the bridge and caller frames are present.
    std::array<std::byte, 64 * 1024> buffer{};
    std::uint64_t remaining = byte_count;
    while (remaining != 0) {
      const auto request = static_cast<DWORD>(std::min<std::uint64_t>(remaining, buffer.size()));
      DWORD read = 0;
      if (!ReadFile(file, buffer.data(), request, &read, nullptr) || read != request ||
          BCryptHashData(
              hash,
              reinterpret_cast<PUCHAR>(buffer.data()),
              read,
              0) < 0) {
        remaining = std::numeric_limits<std::uint64_t>::max();
        break;
      }
      remaining -= read;
    }
    if (remaining != 0 ||
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

bool is_zero(const Digest& digest) noexcept {
  return std::all_of(digest.begin(), digest.end(), [](const std::byte value) {
    return value == std::byte{0};
  });
}

std::optional<std::uint32_t> rva_to_file_offset(
    const IMAGE_NT_HEADERS64& nt,
    const IMAGE_SECTION_HEADER* sections,
    const std::uint32_t rva) noexcept {
  if (rva < nt.OptionalHeader.SizeOfHeaders) {
    return rva;
  }
  for (std::uint16_t index = 0; index < nt.FileHeader.NumberOfSections; ++index) {
    const auto& section = sections[index];
    const auto extent = std::max(section.Misc.VirtualSize, section.SizeOfRawData);
    if (rva >= section.VirtualAddress && rva - section.VirtualAddress < extent) {
      return section.PointerToRawData + (rva - section.VirtualAddress);
    }
  }
  return std::nullopt;
}

[[maybe_unused]] bool verify_pe_and_codeview(
    const HANDLE file,
    const std::uint64_t file_bytes) noexcept {
  const HANDLE mapping = CreateFileMappingW(file, nullptr, PAGE_READONLY, 0, 0, nullptr);
  if (mapping == nullptr) {
    return false;
  }
  const auto* base = static_cast<const std::byte*>(MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0));
  if (base == nullptr) {
    CloseHandle(mapping);
    return false;
  }
  bool valid = false;
  do {
    if (file_bytes < sizeof(IMAGE_DOS_HEADER)) {
      break;
    }
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(base);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew < 0 ||
        static_cast<std::uint64_t>(dos->e_lfanew) + sizeof(IMAGE_NT_HEADERS64) > file_bytes) {
      break;
    }
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS64*>(base + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE || nt->FileHeader.Machine != IMAGE_FILE_MACHINE_AMD64 ||
        nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC ||
        nt->OptionalHeader.SizeOfImage != kPeSizeOfImage) {
      break;
    }
    const auto* sections = IMAGE_FIRST_SECTION(nt);
    const auto section_end = reinterpret_cast<const std::byte*>(
        sections + nt->FileHeader.NumberOfSections);
    if (section_end > base + file_bytes) {
      break;
    }
    const auto& directory =
        nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG];
    const auto debug_offset = rva_to_file_offset(*nt, sections, directory.VirtualAddress);
    if (!debug_offset.has_value() || directory.Size < sizeof(IMAGE_DEBUG_DIRECTORY) ||
        static_cast<std::uint64_t>(*debug_offset) + directory.Size > file_bytes) {
      break;
    }
    const auto count = directory.Size / sizeof(IMAGE_DEBUG_DIRECTORY);
    const auto* debug = reinterpret_cast<const IMAGE_DEBUG_DIRECTORY*>(base + *debug_offset);
    for (std::uint32_t index = 0; index < count; ++index) {
      if (debug[index].Type != IMAGE_DEBUG_TYPE_CODEVIEW || debug[index].SizeOfData < 24 ||
          static_cast<std::uint64_t>(debug[index].PointerToRawData) + debug[index].SizeOfData >
              file_bytes) {
        continue;
      }
      const auto* codeview = base + debug[index].PointerToRawData;
      if (std::memcmp(codeview, "RSDS", 4) == 0 &&
          std::memcmp(codeview + 4, kCodeViewGuidRsds.data(), kCodeViewGuidRsds.size()) == 0) {
        std::uint32_t age = 0;
        std::memcpy(&age, codeview + 20, sizeof(age));
        if (age == kCodeViewAge) {
          valid = true;
          break;
        }
      }
    }
  } while (false);
  UnmapViewOfFile(base);
  CloseHandle(mapping);
  return valid;
}

bool verify_loaded_image(const void* image_base) noexcept {
  if (image_base == nullptr) {
    return false;
  }
  MEMORY_BASIC_INFORMATION region{};
  if (VirtualQuery(image_base, &region, sizeof(region)) != sizeof(region) ||
      region.State != MEM_COMMIT ||
      (region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0) {
    return false;
  }
  const auto protection = region.Protect & 0xffu;
  if (protection != PAGE_READONLY && protection != PAGE_READWRITE &&
      protection != PAGE_WRITECOPY && protection != PAGE_EXECUTE_READ &&
      protection != PAGE_EXECUTE_READWRITE && protection != PAGE_EXECUTE_WRITECOPY) {
    return false;
  }
  const auto base = reinterpret_cast<std::uintptr_t>(image_base);
  const auto region_base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
  if (region.RegionSize < 4096 || region_base > base ||
      base - region_base > region.RegionSize - 4096) {
    return false;
  }
  std::array<std::byte, 4096> headers{};
  std::memcpy(headers.data(), image_base, headers.size());
  const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(headers.data());
  if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew < 0 ||
      static_cast<std::size_t>(dos->e_lfanew) + sizeof(IMAGE_NT_HEADERS64) > headers.size()) {
    return false;
  }
  const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS64*>(headers.data() + dos->e_lfanew);
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  return nt->Signature == IMAGE_NT_SIGNATURE &&
         nt->FileHeader.Machine == IMAGE_FILE_MACHINE_AMD64 &&
         nt->OptionalHeader.Magic == IMAGE_NT_OPTIONAL_HDR64_MAGIC &&
         nt->OptionalHeader.SizeOfImage != 0;
#else
  return nt->Signature == IMAGE_NT_SIGNATURE &&
         nt->FileHeader.Machine == IMAGE_FILE_MACHINE_AMD64 &&
         nt->OptionalHeader.Magic == IMAGE_NT_OPTIONAL_HDR64_MAGIC &&
         nt->OptionalHeader.SizeOfImage == kPeSizeOfImage;
#endif
}

bool valid_json(std::span<const std::byte> json) noexcept {
  if (json.empty() || json.size() > kJsonPayloadLimit || json.front() != std::byte{'{'}) {
    return false;
  }
  return std::find(json.begin(), json.end(), std::byte{0}) == json.end();
}

}  // namespace

struct CaptureSession::Impl final {
  HANDLE file{INVALID_HANDLE_VALUE};
  HANDLE executable_directory{INVALID_HANDLE_VALUE};
  CaptureError last_error{CaptureError::invalid_state};
  std::uintptr_t image_base{};
  std::uint64_t stream_bytes{};
  std::uint64_t records{};
  std::vector<std::uint32_t> pointer_rvas;
  bool opened{};
  bool open_attempted{};
  bool engine_property_seen{};
  bool active_bind{};
  bool bind_seen{};
  std::uint32_t active_callback{};
  std::int32_t active_order{};
  std::uint32_t active_token{};
  std::uint32_t next_callback{};
  bool registry_support_seen{};
  std::uint32_t final_state_count{};
  bool build_seen{};
  std::uint32_t frontend_config_mask{};
  std::uint32_t frontend_boundary_count{};
  std::optional<Digest> frontend_config_digest;
  bool sealed{};

  ~Impl() {
    if (file != INVALID_HANDLE_VALUE) {
      CloseHandle(file);
    }
    if (executable_directory != INVALID_HANDLE_VALUE) {
      CloseHandle(executable_directory);
    }
  }

  CaptureError fail(const CaptureError error) noexcept {
    last_error = error;
    return error;
  }

  CaptureError append_record(
      const RecordKind kind,
      const std::span<const std::byte> payload) noexcept {
    if (!opened || sealed || last_error != CaptureError::ok) {
      return fail(CaptureError::invalid_state);
    }
    if (payload.size() > kMaxPayloadBytes) {
      return fail(CaptureError::size_limit);
    }
    if (records >= kMaxRecords) {
      return fail(CaptureError::record_limit);
    }
    const auto added = kRecordHeaderBytes + payload.size();
    if (stream_bytes > kMaxCaptureBytes - kFooterBytes - added) {
      return fail(CaptureError::size_limit);
    }
    std::vector<std::byte> header;
    header.reserve(kRecordHeaderBytes);
    put_u16(header, static_cast<std::uint16_t>(kind));
    put_u16(header, kSchemaVersion);
    put_u32(header, 0);
    put_u32(header, static_cast<std::uint32_t>(payload.size()));
    put_u32(header, 0);
    put_u64(header, records);
    if (!write_all(file, header) || !write_all(file, payload)) {
      return fail(CaptureError::io_error);
    }
    stream_bytes += added;
    ++records;
    return CaptureError::ok;
  }

  CaptureError append_bind(
      const std::uint32_t phase,
      const std::uint32_t callback_ordinal,
      const std::int32_t bind_order,
      const std::uint32_t callback_pointer_token,
      const RegistryCounts& counts,
      const Digest& registry_sha256) noexcept {
    if (callback_pointer_token >= pointer_rvas.size() || is_zero(registry_sha256)) {
      return fail(CaptureError::invalid_argument);
    }
    if (phase == 1) {
      if (active_bind || final_state_count != 0 || build_seen || !engine_property_seen ||
          callback_ordinal != next_callback) {
        return fail(CaptureError::invalid_state);
      }
      active_bind = true;
      active_callback = callback_ordinal;
      active_order = bind_order;
      active_token = callback_pointer_token;
    } else if (phase == 2) {
      if (!active_bind || callback_ordinal != active_callback || bind_order != active_order ||
          callback_pointer_token != active_token) {
        return fail(CaptureError::invalid_state);
      }
    } else {
      return fail(CaptureError::invalid_argument);
    }
    std::vector<std::byte> payload;
    payload.reserve(88);
    put_u32(payload, callback_ordinal);
    put_u32(payload, phase);
    put_i32(payload, bind_order);
    put_u32(payload, callback_pointer_token);
    put_u32(payload, phase == 1 ? kRvaBindCallbackCall : kRvaBindCallbackReturn);
    put_u32(payload, 0);
    for (const auto value : {counts.types, counts.functions, counts.object_properties,
                             counts.global_properties, counts.enum_values, counts.funcdefs,
                             counts.typedefs, counts.total_registrations}) {
      put_u32(payload, value);
    }
    put_array(payload, registry_sha256);
    const auto result = append_record(RecordKind::bind_callback, payload);
    if (result == CaptureError::ok && phase == 2) {
      active_bind = false;
      bind_seen = true;
      ++next_callback;
    }
    return result;
  }
};

CaptureSession::CaptureSession() noexcept : impl_(new (std::nothrow) Impl()) {}
CaptureSession::~CaptureSession() = default;
CaptureSession::CaptureSession(CaptureSession&&) noexcept = default;
CaptureSession& CaptureSession::operator=(CaptureSession&&) noexcept = default;

CaptureError CaptureSession::open_pinned(
    const std::filesystem::path& executable_path,
    const std::filesystem::path& output_path,
    const void* primary_image_base,
    const std::uint64_t observed_steam_build_id,
    const GuidBytes& capture_id) noexcept {
  if (!impl_ || impl_->open_attempted) {
    return impl_ ? impl_->fail(CaptureError::invalid_argument) : CaptureError::invalid_state;
  }
  impl_->open_attempted = true;
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  constexpr std::uint64_t expected_observed_build_id = 0xf17e'2453'9464'0001ull;
#else
  constexpr std::uint64_t expected_observed_build_id = kSteamBuildId;
#endif
  if (observed_steam_build_id != expected_observed_build_id ||
      std::all_of(capture_id.begin(), capture_id.end(), [](const std::byte value) {
        return value == std::byte{0};
      })) {
    return impl_->fail(CaptureError::invalid_argument);
  }
  try {
    std::array<wchar_t, 32768> loaded_path{};
    const auto loaded_length = GetModuleFileNameW(
        static_cast<HMODULE>(const_cast<void*>(primary_image_base)),
        loaded_path.data(),
        static_cast<DWORD>(loaded_path.size()));
    if (loaded_length == 0 || loaded_length == loaded_path.size() ||
        !verify_loaded_image(primary_image_base)) {
      return impl_->fail(CaptureError::wrong_target);
    }
    auto source = detail::open_pinned_source(
        executable_path,
        std::filesystem::path(loaded_path.data(), loaded_path.data() + loaded_length));
    if (source.error != CaptureError::ok) {
      return impl_->fail(source.error);
    }
    LARGE_INTEGER file_size{};
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
    const bool target_ok = GetFileSizeEx(source.executable.get(), &file_size) &&
                           file_size.QuadPart > 0;
#else
    Digest executable_sha{};
    const bool target_ok = GetFileSizeEx(source.executable.get(), &file_size) &&
                           static_cast<std::uint64_t>(file_size.QuadPart) == kExecutableBytes &&
                           hash_handle_prefix(
                               source.executable.get(), kExecutableBytes, {}, executable_sha) &&
                           executable_sha == kExecutableSha256 &&
                           verify_pe_and_codeview(source.executable.get(), kExecutableBytes);
#endif
    if (!target_ok) {
      return impl_->fail(CaptureError::wrong_target);
    }
    auto output = detail::create_pinned_output(output_path, source.executable_directory.get());
    if (output.error != CaptureError::ok) {
      return impl_->fail(output.error);
    }
    impl_->file = output.output.release();
    impl_->executable_directory = source.executable_directory.release();
    std::vector<std::byte> header;
    header.reserve(kHeaderBytes);
    put_array(header, kCaptureMagic);
    put_u16(header, kSchemaVersion);
    put_u16(header, static_cast<std::uint16_t>(kHeaderBytes));
    put_u32(header, 0);
    put_u32(header, kSteamAppId);
    put_u64(header, kSteamBuildId);
    put_u32(header, kAngelScriptVersion);
    put_u64(header, kExecutableBytes);
    put_array(header, kExecutableSha256);
    put_array(header, kCodeViewGuidRsds);
    put_u32(header, kCodeViewAge);
    put_array(header, capture_id);
    put_u32(header, 0);
    if (header.size() != kHeaderBytes || !write_all(impl_->file, header)) {
      return impl_->fail(CaptureError::io_error);
    }
    impl_->image_base = reinterpret_cast<std::uintptr_t>(primary_image_base);
    impl_->stream_bytes = header.size();
    impl_->opened = true;
    impl_->last_error = CaptureError::ok;
    return CaptureError::ok;
  } catch (...) {
    return impl_->fail(CaptureError::invalid_argument);
  }
}

CaptureError CaptureSession::append_engine_property(
    const std::uint32_t property_id,
    const std::uint64_t value,
    const std::uint32_t observation_rva) noexcept {
  if (!impl_ || impl_->active_bind || impl_->bind_seen || property_id == 0 ||
      observation_rva != kRvaSetEngineProperty) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  std::vector<std::byte> payload;
  payload.reserve(24);
  put_u32(payload, property_id);
  put_u32(payload, 0);
  put_u64(payload, value);
  put_u32(payload, observation_rva);
  put_u32(payload, 0);
  const auto result = impl_->append_record(RecordKind::engine_property, payload);
  if (result == CaptureError::ok) {
    impl_->engine_property_seen = true;
  }
  return result;
}

CaptureError CaptureSession::intern_primary_image_pointer(
    const void* pointer,
    std::uint32_t& token_out) noexcept {
  if (!impl_ || !impl_->opened || impl_->final_state_count != 0 || impl_->build_seen ||
      pointer == nullptr) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  const auto address = reinterpret_cast<std::uintptr_t>(pointer);
  if (address <= impl_->image_base || address - impl_->image_base >= kPeSizeOfImage) {
    return impl_->fail(CaptureError::pointer_outside_primary_image);
  }
  const auto rva = static_cast<std::uint32_t>(address - impl_->image_base);
  const auto found = std::find(impl_->pointer_rvas.begin(), impl_->pointer_rvas.end(), rva);
  if (found != impl_->pointer_rvas.end()) {
    token_out = static_cast<std::uint32_t>(found - impl_->pointer_rvas.begin());
    return CaptureError::ok;
  }
  if (impl_->pointer_rvas.size() >= std::numeric_limits<std::uint32_t>::max()) {
    return impl_->fail(CaptureError::size_limit);
  }
  token_out = static_cast<std::uint32_t>(impl_->pointer_rvas.size());
  std::vector<std::byte> payload;
  payload.reserve(12);
  put_u32(payload, token_out);
  put_u32(payload, rva);
  put_u32(payload, 0);
  const auto result = impl_->append_record(RecordKind::pointer_token, payload);
  if (result == CaptureError::ok) {
    impl_->pointer_rvas.push_back(rva);
  }
  return result;
}

CaptureError CaptureSession::append_bind_begin(
    const std::uint32_t callback_ordinal,
    const std::int32_t bind_order,
    const std::uint32_t callback_pointer_token,
    const RegistryCounts& counts,
    const Digest& registry_sha256) noexcept {
  return impl_ ? impl_->append_bind(
                     1, callback_ordinal, bind_order, callback_pointer_token, counts,
                     registry_sha256)
               : CaptureError::invalid_state;
}

CaptureError CaptureSession::append_bind_end(
    const std::uint32_t callback_ordinal,
    const std::int32_t bind_order,
    const std::uint32_t callback_pointer_token,
    const RegistryCounts& counts,
    const Digest& registry_sha256) noexcept {
  return impl_ ? impl_->append_bind(
                     2, callback_ordinal, bind_order, callback_pointer_token, counts,
                     registry_sha256)
               : CaptureError::invalid_state;
}

CaptureError CaptureSession::append_registry_delta_json(
    const std::span<const std::byte> utf8_json) noexcept {
  if (!impl_ || !impl_->active_bind || !valid_json(utf8_json)) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  return impl_->append_record(RecordKind::registry_delta_json, utf8_json);
}

CaptureError CaptureSession::append_post_bind_mutation_json(
    const std::span<const std::byte> utf8_json) noexcept {
  if (!impl_ || !impl_->active_bind || !valid_json(utf8_json)) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  return impl_->append_record(RecordKind::post_bind_mutation_json, utf8_json);
}

CaptureError CaptureSession::append_registry_support_json(
    const std::span<const std::byte> utf8_json) noexcept {
  if (!impl_ || impl_->active_bind || !impl_->bind_seen || impl_->registry_support_seen ||
      impl_->final_state_count != 0 || !valid_json(utf8_json)) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  const auto result = impl_->append_record(RecordKind::registry_support_json, utf8_json);
  if (result == CaptureError::ok) {
    impl_->registry_support_seen = true;
  }
  return result;
}

CaptureError CaptureSession::append_final_post_bind_state_json(
    const std::span<const std::byte> utf8_json) noexcept {
  if (!impl_ || impl_->active_bind || !impl_->registry_support_seen || impl_->build_seen ||
      !valid_json(utf8_json)) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  const auto result = impl_->append_record(RecordKind::final_post_bind_state_json, utf8_json);
  if (result == CaptureError::ok) {
    ++impl_->final_state_count;
  }
  return result;
}

CaptureError CaptureSession::append_build_jit(const BuildJitFact& fact) noexcept {
  if (!impl_ || impl_->active_bind || !impl_->registry_support_seen ||
      impl_->final_state_count == 0 || impl_->build_seen ||
      fact.build_identifier != kBuildIdentifier || !fact.shipping_cache_matches ||
      fact.jit_database_cleared || fact.precompiled_guid != kPrecompiledGuid ||
      fact.as_reference_debugging || !fact.fork_opcode_table_201_212_present ||
      fact.reference_debug_opcodes_emittable ||
      fact.resolve_object_ptr_callback_registered ||
      fact.get_build_identifier_rva != kRvaGetBuildIdentifier ||
      fact.get_static_jit_info_rva != kRvaGetStaticJitInfo ||
      (fact.jit_info_present &&
       (!fact.jit_guid_matches || fact.compiled_jit_guid != fact.precompiled_guid)) ||
      (!fact.jit_info_present &&
       (fact.jit_guid_matches || fact.compiled_jit_guid != GuidBytes{}))) {
    return impl_ ? impl_->fail(CaptureError::wrong_target) : CaptureError::invalid_state;
  }
  std::uint32_t flags = fact.jit_info_present ? 1u : 0u;
  flags |= fact.jit_guid_matches ? 2u : 0u;
  flags |= fact.jit_database_cleared ? 4u : 0u;
  flags |= fact.shipping_cache_matches ? 8u : 0u;
  flags |= fact.as_reference_debugging ? 16u : 0u;
  flags |= fact.fork_opcode_table_201_212_present ? 32u : 0u;
  flags |= fact.reference_debug_opcodes_emittable ? 64u : 0u;
  flags |= fact.resolve_object_ptr_callback_registered ? 128u : 0u;
  std::vector<std::byte> payload;
  payload.reserve(48);
  put_u32(payload, fact.build_identifier);
  put_u32(payload, flags);
  put_array(payload, fact.precompiled_guid);
  put_array(payload, fact.compiled_jit_guid);
  put_u32(payload, fact.get_build_identifier_rva);
  put_u32(payload, fact.get_static_jit_info_rva);
  const auto result = impl_->append_record(RecordKind::build_jit, payload);
  if (result == CaptureError::ok) {
    impl_->build_seen = true;
  }
  return result;
}

CaptureError CaptureSession::append_frontend_config_json(
    const std::uint32_t config_kind,
    const std::span<const std::byte> utf8_json) noexcept {
  if (!impl_ || !impl_->build_seen || impl_->frontend_boundary_count >= 3 || config_kind < 1 ||
      config_kind > 3 || (impl_->frontend_config_mask & (1u << config_kind)) != 0 ||
      !valid_json(utf8_json) || utf8_json.size() > kMaxPayloadBytes - 4) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  std::vector<std::byte> payload;
  payload.reserve(4 + utf8_json.size());
  put_u32(payload, config_kind);
  payload.insert(payload.end(), utf8_json.begin(), utf8_json.end());
  const auto result = impl_->append_record(RecordKind::frontend_config_json, payload);
  if (result == CaptureError::ok) {
    impl_->frontend_config_mask |= 1u << config_kind;
  }
  return result;
}

CaptureError CaptureSession::append_frontend_boundary(
    const FrontendBoundary& boundary) noexcept {
  if (!impl_ || !impl_->build_seen || impl_->frontend_config_mask != 0x0eu ||
      impl_->frontend_boundary_count >= 3 || is_zero(boundary.config_sha256)) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  const auto step = impl_->frontend_boundary_count;
  const bool valid =
      (step == 0 && boundary.kind == FrontendBoundaryKind::initial_compile_enter &&
       boundary.observation_rva == kRvaInitialCompileEnter && boundary.module_count == 0 &&
       is_zero(boundary.output_sha256)) ||
      (step == 1 &&
       ((boundary.kind == FrontendBoundaryKind::precompiled_descriptors_requested &&
         boundary.observation_rva == kRvaPrecompiledDescriptorsRequested &&
         boundary.module_count != 0 && !is_zero(boundary.input_sha256) &&
         !is_zero(boundary.output_sha256)) ||
        (boundary.kind == FrontendBoundaryKind::preprocessor_constructed &&
         boundary.observation_rva == kRvaPreprocessorConstructed && boundary.module_count == 0 &&
         is_zero(boundary.input_sha256) && is_zero(boundary.output_sha256)))) ||
      (step == 2 && boundary.kind == FrontendBoundaryKind::initial_compile_return &&
       boundary.observation_rva == kRvaInitialCompileReturn && boundary.module_count != 0 &&
       boundary.result_code == 0 && !is_zero(boundary.output_sha256));
  if (!valid ||
      (impl_->frontend_config_digest.has_value() &&
       *impl_->frontend_config_digest != boundary.config_sha256)) {
    return impl_->fail(CaptureError::invalid_argument);
  }
  std::vector<std::byte> payload;
  payload.reserve(112);
  put_u32(payload, static_cast<std::uint32_t>(boundary.kind));
  put_u32(payload, boundary.observation_rva);
  put_u32(payload, boundary.module_count);
  put_i32(payload, boundary.result_code);
  put_array(payload, boundary.config_sha256);
  put_array(payload, boundary.input_sha256);
  put_array(payload, boundary.output_sha256);
  const auto result = impl_->append_record(RecordKind::frontend_boundary, payload);
  if (result == CaptureError::ok) {
    impl_->frontend_config_digest = boundary.config_sha256;
    ++impl_->frontend_boundary_count;
  }
  return result;
}

CaptureError CaptureSession::seal() noexcept {
  if (!impl_ || !impl_->opened || impl_->sealed || impl_->active_bind || !impl_->build_seen ||
      impl_->frontend_config_mask != 0x0eu || impl_->frontend_boundary_count != 3 ||
      impl_->last_error != CaptureError::ok) {
    return impl_ ? impl_->fail(CaptureError::invalid_state) : CaptureError::invalid_state;
  }
  Digest digest{};
  if (!FlushFileBuffers(impl_->file) ||
      !hash_handle_prefix(impl_->file, impl_->stream_bytes, kHashDomain, digest)) {
    return impl_->fail(CaptureError::crypto_error);
  }
  LARGE_INTEGER end{};
  end.QuadPart = static_cast<LONGLONG>(impl_->stream_bytes);
  if (!SetFilePointerEx(impl_->file, end, nullptr, FILE_BEGIN)) {
    return impl_->fail(CaptureError::io_error);
  }
  std::vector<std::byte> footer;
  footer.reserve(kFooterBytes);
  put_array(footer, kFooterMagic);
  put_u64(footer, impl_->records);
  put_u64(footer, impl_->stream_bytes);
  put_array(footer, digest);
  put_u32(footer, kSchemaVersion);
  put_u32(footer, 0);
  if (footer.size() != kFooterBytes || !write_all(impl_->file, footer) ||
      !FlushFileBuffers(impl_->file)) {
    return impl_->fail(CaptureError::io_error);
  }
  impl_->sealed = true;
  return CaptureError::ok;
}

CaptureError CaptureSession::status() const noexcept {
  return impl_ ? impl_->last_error : CaptureError::invalid_state;
}

std::uint64_t CaptureSession::record_count() const noexcept {
  return impl_ ? impl_->records : 0;
}

}  // namespace gore_as_capture::v1
