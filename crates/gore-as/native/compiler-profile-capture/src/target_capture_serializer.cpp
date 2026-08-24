#include "target_capture_serializer.hpp"

#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::size_t kMaximumJsonBytes = 256u * 1024u * 1024u;
constexpr std::size_t kMaximumTextBytes = 64u * 1024u;
// The explicit trailing NUL is part of the Rust registry hash domains.  A plain
// string_view constructed from a C string would silently omit it.
constexpr char kCallableHashDomainBytes[] = "gore-as-host-stub-callable-v1\0";
constexpr char kObjectHashDomainBytes[] = "gore-as-host-stub-object-v1\0";
constexpr std::string_view kCallableHashDomain{
    kCallableHashDomainBytes, sizeof(kCallableHashDomainBytes) - 1};
constexpr std::string_view kObjectHashDomain{
    kObjectHashDomainBytes, sizeof(kObjectHashDomainBytes) - 1};

constexpr std::array<std::string_view, 9> kCallConventions{
    "cdecl",          "stdcall",              "thiscall_as_global",
    "thiscall",       "cdecl_object_last",    "cdecl_object_first",
    "generic",        "thiscall_object_last", "thiscall_object_first"};
constexpr std::array<std::string_view, 14> kObjectBehaviours{
    "construct",       "list_construct", "destruct",       "factory",
    "list_factory",    "add_ref",        "release",        "get_weakref_flag",
    "template_callback", "get_ref_count",   "set_gc_flag",    "get_gc_flag",
    "enum_refs",       "release_refs"};
constexpr std::array<std::string_view, 9> kTemplateAdapters{
    "t_array",           "t_map",              "t_set",
    "t_optional",        "t_subclass_of",      "t_object_ptr",
    "t_weak_object_ptr", "t_soft_object_ptr",  "t_soft_class_ptr"};

template <std::size_t Size>
bool is_closed_value(
    const std::string_view value,
    const std::array<std::string_view, Size>& values) noexcept {
  return std::find(values.begin(), values.end(), value) != values.end();
}

bool valid_alignment(const std::uint32_t value) noexcept {
  return value != 0 && value <= 4096 && (value & (value - 1)) == 0;
}

bool valid_utf8(const std::string_view value) noexcept {
  if (value.size() > kMaximumTextBytes || value.find('\0') != std::string_view::npos) return false;
  std::size_t cursor = 0;
  while (cursor < value.size()) {
    const auto lead = static_cast<unsigned char>(value[cursor]);
    if (lead < 0x80) {
      ++cursor;
      continue;
    }
    std::size_t continuation = 0;
    std::uint32_t scalar = 0;
    if ((lead & 0xe0) == 0xc0) {
      continuation = 1;
      scalar = lead & 0x1f;
    } else if ((lead & 0xf0) == 0xe0) {
      continuation = 2;
      scalar = lead & 0x0f;
    } else if ((lead & 0xf8) == 0xf0) {
      continuation = 3;
      scalar = lead & 0x07;
    } else {
      return false;
    }
    if (cursor + continuation >= value.size()) return false;
    for (std::size_t index = 1; index <= continuation; ++index) {
      const auto byte = static_cast<unsigned char>(value[cursor + index]);
      if ((byte & 0xc0) != 0x80) return false;
      scalar = (scalar << 6) | (byte & 0x3f);
    }
    if ((continuation == 1 && scalar < 0x80) ||
        (continuation == 2 && scalar < 0x800) ||
        (continuation == 3 && scalar < 0x10000) || scalar > 0x10ffff ||
        (scalar >= 0xd800 && scalar <= 0xdfff)) {
      return false;
    }
    cursor += continuation + 1;
  }
  return true;
}

class Json final {
 public:
  bool raw(const std::string_view value) {
    if (value.size() > kMaximumJsonBytes - std::min(output_.size(), kMaximumJsonBytes)) {
      return false;
    }
    output_.append(value);
    return true;
  }
  bool string(const std::string_view value) {
    if (!valid_utf8(value) || !raw("\"")) return false;
    constexpr char hex[] = "0123456789abcdef";
    for (const unsigned char byte : value) {
      switch (byte) {
        case '"':
          if (!raw("\\\"")) return false;
          break;
        case '\\':
          if (!raw("\\\\")) return false;
          break;
        case '\b':
          if (!raw("\\b")) return false;
          break;
        case '\f':
          if (!raw("\\f")) return false;
          break;
        case '\n':
          if (!raw("\\n")) return false;
          break;
        case '\r':
          if (!raw("\\r")) return false;
          break;
        case '\t':
          if (!raw("\\t")) return false;
          break;
        default:
          if (byte < 0x20) {
            std::array<char, 6> escaped{'\\', 'u', '0', '0', hex[byte >> 4], hex[byte & 0x0f]};
            if (!raw(std::string_view(escaped.data(), escaped.size()))) return false;
          } else if (!raw(std::string_view(reinterpret_cast<const char*>(&byte), 1))) {
            return false;
          }
      }
    }
    return raw("\"");
  }
  bool u32(const std::uint32_t value) { return raw(std::to_string(value)); }
  bool i32(const std::int32_t value) { return raw(std::to_string(value)); }
  bool u64(const std::uint64_t value) { return raw(std::to_string(value)); }
  bool boolean(const bool value) { return raw(value ? "true" : "false"); }
  std::string take() && { return std::move(output_); }

 private:
  std::string output_;
};

bool sha256_witness_set(
    const HostStubDescriptorKind kind,
    const std::vector<std::string>& witnesses,
    std::array<std::uint8_t, 32>& digest) noexcept {
  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  DWORD object_bytes = 0;
  DWORD result_bytes = 0;
  std::vector<std::uint8_t> object;
  const auto close = [&] {
    if (hash != nullptr) (void)BCryptDestroyHash(hash);
    if (algorithm != nullptr) (void)BCryptCloseAlgorithmProvider(algorithm, 0);
  };
  if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0 ||
      BCryptGetProperty(
          algorithm,
          BCRYPT_OBJECT_LENGTH,
          reinterpret_cast<PUCHAR>(&object_bytes),
          sizeof(object_bytes),
          &result_bytes,
          0) < 0 ||
      result_bytes != sizeof(object_bytes) || object_bytes == 0 || object_bytes > 64 * 1024) {
    close();
    return false;
  }
  try {
    object.resize(object_bytes);
  } catch (...) {
    close();
    return false;
  }
  if (BCryptCreateHash(
          algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
    close();
    return false;
  }
  const auto append = [&](const void* bytes, const std::size_t size) {
    return size <= std::numeric_limits<ULONG>::max() &&
           BCryptHashData(
               hash,
               const_cast<PUCHAR>(static_cast<const UCHAR*>(bytes)),
               static_cast<ULONG>(size),
               0) >= 0;
  };
  const auto domain =
      kind == HostStubDescriptorKind::callable ? kCallableHashDomain : kObjectHashDomain;
  const std::uint32_t count = static_cast<std::uint32_t>(witnesses.size());
  bool ok = append(domain.data(), domain.size()) && append(&count, sizeof(count));
  for (const auto& witness : witnesses) {
    const std::uint32_t length = static_cast<std::uint32_t>(witness.size());
    ok = ok && append(&length, sizeof(length)) && append(witness.data(), witness.size());
  }
  ok = ok && BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) >= 0;
  close();
  return ok;
}

bool hex_digest(Json& json, const std::array<std::uint8_t, 32>& digest) {
  constexpr char hex[] = "0123456789abcdef";
  std::array<char, 64> output{};
  for (std::size_t index = 0; index < digest.size(); ++index) {
    output[index * 2] = hex[digest[index] >> 4];
    output[index * 2 + 1] = hex[digest[index] & 0xf];
  }
  return json.string(std::string_view(output.data(), output.size()));
}

bool context_json(Json& json, const RegistrationContextFinalState& context) {
  return !context.has_config_group && valid_utf8(context.name_space) &&
         json.raw("{\"namespace\":") && json.string(context.name_space) &&
         json.raw(",\"config_group\":null,\"access_mask\":") &&
         json.u32(context.access_mask) && json.raw("}");
}

bool fixed_operations_json(Json& json, const FixedTypeOperationsProjection& value) {
  return valid_alignment(value.value_alignment) &&
         json.raw("{\"can_create_property\":") &&
         json.boolean(value.can_create_property) && json.raw(",\"never_requires_gc\":") &&
         json.boolean(value.never_requires_gc) && json.raw(",\"requires_property\":") &&
         json.boolean(value.requires_property) && json.raw(",\"can_be_template_subtype\":") &&
         json.boolean(value.can_be_template_subtype) && json.raw(",\"can_construct\":") &&
         json.boolean(value.can_construct) && json.raw(",\"need_construct\":") &&
         json.boolean(value.need_construct) && json.raw(",\"can_destruct\":") &&
         json.boolean(value.can_destruct) && json.raw(",\"need_destruct\":") &&
         json.boolean(value.need_destruct) && json.raw(",\"can_copy\":") &&
         json.boolean(value.can_copy) && json.raw(",\"need_copy\":") &&
         json.boolean(value.need_copy) && json.raw(",\"can_compare\":") &&
         json.boolean(value.can_compare) && json.raw(",\"can_hash_value\":") &&
         json.boolean(value.can_hash_value) && json.raw(",\"value_size\":") &&
         json.u32(value.value_size) && json.raw(",\"value_alignment\":") &&
         json.u32(value.value_alignment) && json.raw(",\"is_object_pointer\":") &&
         json.boolean(value.is_object_pointer) && json.raw("}");
}

bool type_operations_json(Json& json, const RegistrationEntryJsonProjection& entry) {
  switch (entry.type_operations) {
    case TypeOperationsJsonKind::unavailable:
      return json.raw("{\"kind\":\"unavailable\"}");
    case TypeOperationsJsonKind::fixed:
      return json.raw("{\"kind\":\"fixed\",\"operations\":") &&
             fixed_operations_json(json, entry.fixed_operations) && json.raw("}");
    case TypeOperationsJsonKind::t_array:
      return json.raw("{\"kind\":\"t_array\"}");
    case TypeOperationsJsonKind::t_map:
      return json.raw("{\"kind\":\"t_map\"}");
    case TypeOperationsJsonKind::t_set:
      return json.raw("{\"kind\":\"t_set\"}");
    case TypeOperationsJsonKind::t_optional:
      return json.raw("{\"kind\":\"t_optional\"}");
  }
  return false;
}

bool witness_field(
    std::string& witness,
    const std::string_view name,
    const std::string_view value) {
  if (!valid_utf8(value)) return false;
  const auto length = std::to_string(value.size());
  const std::size_t extra = name.size() + length.size() + value.size() + 3;
  if (extra > kMaximumTextBytes - std::min(witness.size(), kMaximumTextBytes)) return false;
  witness.append(name);
  witness.push_back('=');
  witness.append(length);
  witness.push_back(':');
  witness.append(value);
  witness.push_back(';');
  return true;
}

bool witness_u32(
    std::string& witness,
    const std::string_view name,
    const std::uint32_t value) {
  return witness_field(witness, name, std::to_string(value));
}

bool callable_witness(
    const RegistrationEntryJsonProjection& entry,
    std::string& witness) {
  witness = "gore.as.capture.callable-use/v1;";
  const char* kind = nullptr;
  switch (entry.kind) {
    case RegistrationEntryJsonKind::object_method:
      kind = "object_method";
      break;
    case RegistrationEntryJsonKind::object_behaviour:
      kind = "object_behaviour";
      break;
    case RegistrationEntryJsonKind::global_function:
      kind = "global_function";
      break;
    default:
      return false;
  }
  if (!is_closed_value(entry.call_convention, kCallConventions) ||
      !witness_field(witness, "kind", kind) ||
      !witness_field(witness, "namespace", entry.context.name_space) ||
      !witness_u32(witness, "access_mask", entry.context.access_mask) ||
      !witness_field(witness, "declaration", entry.declaration) ||
      !witness_field(witness, "call_convention", entry.call_convention) ||
      !witness_u32(
          witness,
          "has_auxiliary_object",
          entry.has_auxiliary_object_stub ? 1u : 0u)) {
    return false;
  }
  if (entry.kind == RegistrationEntryJsonKind::object_method ||
      entry.kind == RegistrationEntryJsonKind::object_behaviour) {
    if (!witness_u32(witness, "owner_trace_type_id", entry.owner_trace_type_id) ||
        !witness_u32(witness, "composite_offset", entry.composite_offset) ||
        !witness_u32(
            witness, "is_composite_indirect", entry.is_composite_indirect ? 1u : 0u)) {
      return false;
    }
  }
  if (entry.kind == RegistrationEntryJsonKind::object_method) {
    return entry.accessor_type <= 0xff &&
           witness_u32(witness, "accessor_type", entry.accessor_type);
  }
  if (entry.kind == RegistrationEntryJsonKind::object_behaviour) {
    return is_closed_value(entry.behaviour, kObjectBehaviours) &&
           (entry.has_template_validation_adapter
                ? is_closed_value(entry.template_validation_adapter, kTemplateAdapters)
                : entry.template_validation_adapter.empty()) &&
           ((entry.behaviour == "template_callback") ==
            entry.has_template_validation_adapter) &&
           witness_field(witness, "behaviour", entry.behaviour) &&
           witness_field(
               witness,
               "template_validation_adapter",
               entry.has_template_validation_adapter ? entry.template_validation_adapter
                                                     : "none");
  }
  return true;
}

bool auxiliary_object_witness(
    const RegistrationEntryJsonProjection& entry,
    std::string& witness) {
  witness = "gore.as.capture.object-use/v1;";
  return witness_field(witness, "role", "auxiliary_object") &&
         witness_u32(witness, "owner_trace_type_id", entry.owner_trace_type_id) &&
         witness_field(witness, "declaration", entry.declaration) &&
         witness_field(witness, "call_convention", entry.call_convention);
}

bool string_factory_object_witness(
    const RegistrationEntryJsonProjection& entry,
    std::string& witness) {
  witness = "gore.as.capture.object-use/v1;";
  return witness_field(witness, "role", "string_factory") &&
         witness_field(witness, "interface", "asIStringFactory") &&
         witness_field(witness, "string_type_declaration", entry.declaration);
}

bool optional_stub(Json& json, const bool present, const std::uint32_t value) {
  return present ? json.u32(value) : value == 0 && json.raw("null");
}

bool entry_prefix(Json& json, const RegistrationEntryJsonProjection& entry, const char* kind) {
  return entry.ordinal == entry.registration_id && json.raw("{\"kind\":\"") &&
         json.raw(kind) && json.raw("\",\"ordinal\":") && json.u32(entry.ordinal) &&
         json.raw(",\"registration_id\":") && json.u32(entry.registration_id) &&
         json.raw(",\"context\":") && context_json(json, entry.context);
}

bool entry_json(Json& json, const RegistrationEntryJsonProjection& entry) {
  const auto declaration = [&] { return valid_utf8(entry.declaration) && json.string(entry.declaration); };
  switch (entry.kind) {
    case RegistrationEntryJsonKind::object_type:
      return entry_prefix(json, entry, "object_type") && json.raw(",\"type_id\":") &&
             json.u32(entry.trace_id) && json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"byte_size\":") && json.u32(entry.byte_size) &&
             json.raw(",\"alignment\":") && json.u32(entry.alignment) &&
             valid_alignment(entry.alignment) && json.raw(",\"flags\":") &&
             json.u32(entry.flags) && json.raw(",\"type_operations\":") &&
             type_operations_json(json, entry) && json.raw("}");
    case RegistrationEntryJsonKind::interface:
    case RegistrationEntryJsonKind::enumeration:
    case RegistrationEntryJsonKind::funcdef: {
      const char* kind = entry.kind == RegistrationEntryJsonKind::interface
                             ? "interface"
                             : (entry.kind == RegistrationEntryJsonKind::enumeration ? "enum"
                                                                                     : "funcdef");
      const bool operations_allowed =
          entry.type_operations == TypeOperationsJsonKind::unavailable ||
          entry.type_operations == TypeOperationsJsonKind::fixed;
      const bool operations_valid =
          operations_allowed &&
          (entry.kind != RegistrationEntryJsonKind::funcdef ||
           entry.type_operations == TypeOperationsJsonKind::unavailable);
      return operations_valid && entry_prefix(json, entry, kind) &&
             json.raw(",\"type_id\":") &&
             json.u32(entry.trace_id) && json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"type_operations\":") && type_operations_json(json, entry) &&
             json.raw("}");
    }
    case RegistrationEntryJsonKind::interface_method:
      return entry_prefix(json, entry, "interface_method") &&
             json.raw(",\"function_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"owner_type_id\":") && json.u32(entry.owner_trace_type_id) &&
             json.raw(",\"declaration\":") && declaration() && json.raw("}");
    case RegistrationEntryJsonKind::object_property:
      return entry_prefix(json, entry, "object_property") &&
             json.raw(",\"property_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"owner_type_id\":") && json.u32(entry.owner_trace_type_id) &&
             json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"byte_offset\":") && json.u32(entry.byte_offset) &&
             json.raw(",\"composite_offset\":") && json.u32(entry.composite_offset) &&
             json.raw(",\"is_composite_indirect\":") &&
             json.boolean(entry.is_composite_indirect) && json.raw(",\"accessor_type\":") &&
             json.u32(entry.accessor_type) && entry.accessor_type <= 0xff &&
             json.raw(",\"is_protected\":") && json.boolean(entry.is_protected) &&
             json.raw("}");
    case RegistrationEntryJsonKind::object_method:
      return is_closed_value(entry.call_convention, kCallConventions) &&
             entry_prefix(json, entry, "object_method") &&
             json.raw(",\"function_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"owner_type_id\":") && json.u32(entry.owner_trace_type_id) &&
             json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"call_convention\":") && json.string(entry.call_convention) &&
             json.raw(",\"callable_stub_id\":") && json.u32(entry.callable_stub_id) &&
             json.raw(",\"auxiliary_object_stub_id\":") &&
             optional_stub(
                 json, entry.has_auxiliary_object_stub, entry.auxiliary_object_stub_id) &&
             json.raw(",\"composite_offset\":") && json.u32(entry.composite_offset) &&
             json.raw(",\"is_composite_indirect\":") &&
             json.boolean(entry.is_composite_indirect) && json.raw(",\"accessor_type\":") &&
             json.u32(entry.accessor_type) && entry.accessor_type <= 0xff && json.raw("}");
    case RegistrationEntryJsonKind::object_behaviour:
      return is_closed_value(entry.behaviour, kObjectBehaviours) &&
             is_closed_value(entry.call_convention, kCallConventions) &&
             (entry.has_template_validation_adapter
                  ? is_closed_value(entry.template_validation_adapter, kTemplateAdapters)
                  : entry.template_validation_adapter.empty()) &&
             ((entry.behaviour == "template_callback") ==
              entry.has_template_validation_adapter) &&
             entry_prefix(json, entry, "object_behaviour") &&
             json.raw(",\"function_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"owner_type_id\":") && json.u32(entry.owner_trace_type_id) &&
             json.raw(",\"behaviour\":") && json.string(entry.behaviour) &&
             json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"call_convention\":") && json.string(entry.call_convention) &&
             json.raw(",\"callable_stub_id\":") && json.u32(entry.callable_stub_id) &&
             json.raw(",\"auxiliary_object_stub_id\":") &&
             optional_stub(
                 json, entry.has_auxiliary_object_stub, entry.auxiliary_object_stub_id) &&
             json.raw(",\"template_validation_adapter\":") &&
             (entry.has_template_validation_adapter
                  ? json.string(entry.template_validation_adapter)
                  : json.raw("null")) &&
             json.raw(",\"composite_offset\":") && json.u32(entry.composite_offset) &&
             json.raw(",\"is_composite_indirect\":") &&
             json.boolean(entry.is_composite_indirect) && json.raw("}");
    case RegistrationEntryJsonKind::global_property:
      return entry_prefix(json, entry, "global_property") &&
             json.raw(",\"property_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"storage_stub_id\":") && json.u32(entry.storage_stub_id) &&
             json.raw("}");
    case RegistrationEntryJsonKind::global_function:
      return is_closed_value(entry.call_convention, kCallConventions) &&
             entry_prefix(json, entry, "global_function") &&
             json.raw(",\"function_id\":") && json.u32(entry.trace_id) &&
             json.raw(",\"declaration\":") && declaration() &&
             json.raw(",\"call_convention\":") && json.string(entry.call_convention) &&
             json.raw(",\"callable_stub_id\":") && json.u32(entry.callable_stub_id) &&
             json.raw(",\"auxiliary_object_stub_id\":") &&
             optional_stub(
                 json, entry.has_auxiliary_object_stub, entry.auxiliary_object_stub_id) &&
             json.raw("}");
    case RegistrationEntryJsonKind::enum_value:
      return entry_prefix(json, entry, "enum_value") &&
             json.raw(",\"owner_type_id\":") && json.u32(entry.owner_trace_type_id) &&
             json.raw(",\"name\":") && json.string(entry.name) && json.raw(",\"value\":") &&
             json.i32(entry.enum_value) && json.raw("}");
    case RegistrationEntryJsonKind::type_alias:
      return entry_prefix(json, entry, "typedef") && json.raw(",\"type_id\":") &&
             json.u32(entry.trace_id) && json.raw(",\"name\":") && json.string(entry.name) &&
             json.raw(",\"target_declaration\":") && json.string(entry.target_declaration) &&
             json.raw("}");
    case RegistrationEntryJsonKind::string_factory:
      return entry_prefix(json, entry, "string_factory") &&
             json.raw(",\"string_type_declaration\":") && declaration() &&
             json.raw(",\"factory_object_stub_id\":") &&
             json.u32(entry.factory_object_stub_id) && json.raw("}");
    case RegistrationEntryJsonKind::default_array_type:
      return entry_prefix(json, entry, "default_array_type") &&
             json.raw(",\"type_declaration\":") && declaration() && json.raw("}");
  }
  return false;
}

bool result_json(
    Json& json,
    const RegistrationEntryJsonKind entry_kind,
    const RegistrationResultJsonProjection& result) {
  const auto& post = result.post_result;
  const auto registration_kind = [&] {
    switch (entry_kind) {
      case RegistrationEntryJsonKind::global_function: return 1u;
      case RegistrationEntryJsonKind::global_property: return 2u;
      case RegistrationEntryJsonKind::object_type: return 3u;
      case RegistrationEntryJsonKind::object_property: return 4u;
      case RegistrationEntryJsonKind::object_method: return 5u;
      case RegistrationEntryJsonKind::object_behaviour: return 6u;
      case RegistrationEntryJsonKind::interface: return 7u;
      case RegistrationEntryJsonKind::interface_method: return 8u;
      case RegistrationEntryJsonKind::string_factory: return 9u;
      case RegistrationEntryJsonKind::default_array_type: return 10u;
      case RegistrationEntryJsonKind::enumeration: return 11u;
      case RegistrationEntryJsonKind::enum_value: return 12u;
      case RegistrationEntryJsonKind::funcdef: return 13u;
      case RegistrationEntryJsonKind::type_alias: return 14u;
    }
    return 0u;
  }();
  if (post.registration_kind != registration_kind) return false;
  const bool owner_required =
      entry_kind == RegistrationEntryJsonKind::interface_method ||
      entry_kind == RegistrationEntryJsonKind::object_property ||
      entry_kind == RegistrationEntryJsonKind::object_method ||
      entry_kind == RegistrationEntryJsonKind::object_behaviour ||
      entry_kind == RegistrationEntryJsonKind::enum_value;
  if (owner_required != result.has_owner_engine_type_id) return false;
  switch (entry_kind) {
    case RegistrationEntryJsonKind::object_type:
    case RegistrationEntryJsonKind::interface:
    case RegistrationEntryJsonKind::enumeration:
    case RegistrationEntryJsonKind::funcdef:
    case RegistrationEntryJsonKind::type_alias: {
      const char* kind = entry_kind == RegistrationEntryJsonKind::object_type
                             ? "object_type"
                             : entry_kind == RegistrationEntryJsonKind::interface
                                   ? "interface"
                                   : entry_kind == RegistrationEntryJsonKind::enumeration
                                         ? "enum"
                                         : entry_kind == RegistrationEntryJsonKind::funcdef
                                               ? "funcdef"
                                               : "typedef";
      return post.semantic == RegistrationResultSemantic::engine_type_id &&
             json.raw("{\"kind\":\"") && json.raw(kind) &&
             json.raw("\",\"engine_type_id\":") && json.u32(post.value) && json.raw("}");
    }
    case RegistrationEntryJsonKind::interface_method:
    case RegistrationEntryJsonKind::object_method:
    case RegistrationEntryJsonKind::object_behaviour: {
      const char* kind = entry_kind == RegistrationEntryJsonKind::interface_method
                             ? "interface_method"
                             : entry_kind == RegistrationEntryJsonKind::object_method
                                   ? "object_method"
                                   : "object_behaviour";
      return post.semantic == RegistrationResultSemantic::engine_function_id &&
             json.raw("{\"kind\":\"") && json.raw(kind) &&
             json.raw("\",\"owner_engine_type_id\":") &&
             json.u32(result.owner_engine_type_id) &&
             json.raw(",\"engine_function_id\":") && json.u32(post.value) && json.raw("}");
    }
    case RegistrationEntryJsonKind::object_property:
      return post.semantic == RegistrationResultSemantic::object_property_index &&
             json.raw("{\"kind\":\"object_property\",\"owner_engine_type_id\":") &&
             json.u32(result.owner_engine_type_id) && json.raw(",\"property_index\":") &&
             json.u32(post.value) && json.raw("}");
    case RegistrationEntryJsonKind::global_property:
      return post.semantic == RegistrationResultSemantic::global_property_index &&
             json.raw("{\"kind\":\"global_property\",\"global_property_index\":") &&
             json.u32(post.value) && json.raw("}");
    case RegistrationEntryJsonKind::global_function:
      return post.semantic == RegistrationResultSemantic::engine_function_id &&
             json.raw("{\"kind\":\"global_function\",\"engine_function_id\":") &&
             json.u32(post.value) && json.raw("}");
    case RegistrationEntryJsonKind::enum_value:
      return post.semantic == RegistrationResultSemantic::enum_value_index &&
             json.raw("{\"kind\":\"enum_value\",\"owner_engine_type_id\":") &&
             json.u32(result.owner_engine_type_id) && json.raw(",\"value_index\":") &&
             json.u32(post.value) && json.raw("}");
    case RegistrationEntryJsonKind::string_factory:
      return post.semantic == RegistrationResultSemantic::installed && post.installed &&
             json.raw("{\"kind\":\"string_factory\",\"installed\":true}");
    case RegistrationEntryJsonKind::default_array_type:
      return post.semantic == RegistrationResultSemantic::installed && post.installed &&
             json.raw("{\"kind\":\"default_array_type\",\"installed\":true}");
  }
  return false;
}

bool final_prefix(Json& json, const std::uint32_t ordinal, const char* kind) {
  return json.raw(
             "{\"schema\":\"gore.as.capture.post-bind-state\",\"schema_version\":1,"
             "\"bind_callback_ordinal\":null,\"state_ordinal\":") &&
         json.u32(ordinal) && json.raw(",\"state\":{\"kind\":\"") && json.raw(kind) &&
         json.raw("\",");
}

bool ids(Json& json, const std::vector<std::uint32_t>& values) {
  if (!json.raw("[")) return false;
  for (std::size_t index = 0; index < values.size(); ++index) {
    if ((index != 0 && !json.raw(",")) || !json.u32(values[index])) return false;
  }
  return json.raw("]");
}

CaptureSerializationError finish_json(Json&& writer, std::string& output) noexcept {
  try {
    auto value = std::move(writer).take();
    if (value.empty() || value.size() > kMaximumJsonBytes) {
      return CaptureSerializationError::limit_exceeded;
    }
    output = std::move(value);
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

}  // namespace

CaptureSerializationError HostStubCatalog::derive_registration_stubs(
    RegistrationEntryJsonProjection& entry,
    const RegistrationStubCapabilities& capabilities) noexcept {
  try {
    auto staged_entry = entry;
    // The production observer is terminal on any derivation failure, so
    // copying the complete, growing catalog for every registration only adds
    // quadratic work without providing a recoverable transaction boundary.
    // Keep the caller-owned entry staged; the sealed output is still emitted
    // atomically only after the whole capture succeeds.
    const auto status =
        derive_registration_stubs_in_place(staged_entry, capabilities);
    if (status != CaptureSerializationError::ok) return status;
    entry = std::move(staged_entry);
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError HostStubCatalog::derive_registration_stubs_in_place(
    RegistrationEntryJsonProjection& entry,
    const RegistrationStubCapabilities& capabilities) noexcept {
  const bool clean_absent =
      (capabilities.has_callable || capabilities.callable_pointer_token == 0) &&
      (capabilities.has_auxiliary_object ||
       capabilities.auxiliary_object_pointer_token == 0) &&
      (capabilities.has_storage ||
       (capabilities.storage_pointer_token == 0 && capabilities.storage_byte_len == 0 &&
        capabilities.storage_alignment == 0)) &&
      (capabilities.has_factory_object || capabilities.factory_object_pointer_token == 0);
  if (!clean_absent) return CaptureSerializationError::invalid_argument;
  try {
    std::string callable;
    std::string object;
    switch (entry.kind) {
      case RegistrationEntryJsonKind::object_method:
      case RegistrationEntryJsonKind::object_behaviour:
      case RegistrationEntryJsonKind::global_function:
        if (!capabilities.has_callable || capabilities.has_storage ||
            capabilities.has_factory_object ||
            (capabilities.has_auxiliary_object !=
             (entry.call_convention == "thiscall_as_global"))) {
          return CaptureSerializationError::invalid_argument;
        }
        entry.has_auxiliary_object_stub = capabilities.has_auxiliary_object;
        entry.auxiliary_object_stub_id = 0;
        if (!callable_witness(entry, callable)) {
          return CaptureSerializationError::invalid_argument;
        }
        {
          const auto status = intern(
              HostStubDescriptorKind::callable,
              capabilities.callable_pointer_token,
              callable,
              0,
              0,
              entry.callable_stub_id);
          if (status != CaptureSerializationError::ok) return status;
        }
        if (capabilities.has_auxiliary_object) {
          if (!auxiliary_object_witness(entry, object)) {
            return CaptureSerializationError::invalid_argument;
          }
          const auto status = intern(
              HostStubDescriptorKind::object,
              capabilities.auxiliary_object_pointer_token,
              object,
              0,
              0,
              entry.auxiliary_object_stub_id);
          if (status != CaptureSerializationError::ok) return status;
        }
        return CaptureSerializationError::ok;
      case RegistrationEntryJsonKind::global_property:
        if (!capabilities.has_storage || capabilities.has_callable ||
            capabilities.has_auxiliary_object || capabilities.has_factory_object) {
          return CaptureSerializationError::invalid_argument;
        }
        return intern(
            HostStubDescriptorKind::storage,
            capabilities.storage_pointer_token,
            {},
            capabilities.storage_byte_len,
            capabilities.storage_alignment,
            entry.storage_stub_id);
      case RegistrationEntryJsonKind::string_factory:
        if (!capabilities.has_factory_object || capabilities.has_callable ||
            capabilities.has_auxiliary_object || capabilities.has_storage ||
            !string_factory_object_witness(entry, object)) {
          return CaptureSerializationError::invalid_argument;
        }
        return intern(
            HostStubDescriptorKind::object,
            capabilities.factory_object_pointer_token,
            object,
            0,
            0,
            entry.factory_object_stub_id);
      case RegistrationEntryJsonKind::object_type:
      case RegistrationEntryJsonKind::interface:
      case RegistrationEntryJsonKind::interface_method:
      case RegistrationEntryJsonKind::object_property:
      case RegistrationEntryJsonKind::enumeration:
      case RegistrationEntryJsonKind::enum_value:
      case RegistrationEntryJsonKind::funcdef:
      case RegistrationEntryJsonKind::type_alias:
      case RegistrationEntryJsonKind::default_array_type:
        return !capabilities.has_callable && !capabilities.has_auxiliary_object &&
                       !capabilities.has_storage && !capabilities.has_factory_object
                   ? CaptureSerializationError::ok
                   : CaptureSerializationError::invalid_argument;
      default:
        return CaptureSerializationError::invalid_argument;
    }
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError HostStubCatalog::intern(
    const HostStubDescriptorKind kind,
    const std::uint32_t pointer_token,
    const std::string_view semantic_witness,
    const std::uint32_t byte_len,
    const std::uint32_t alignment,
    std::uint32_t& stub_id) noexcept {
  if (pointer_token == std::numeric_limits<std::uint32_t>::max() ||
      (kind != HostStubDescriptorKind::storage && !valid_utf8(semantic_witness)) ||
      (kind == HostStubDescriptorKind::storage &&
       (byte_len == 0 || byte_len > 64u * 1024u * 1024u ||
        !valid_alignment(alignment)))) {
    return CaptureSerializationError::invalid_argument;
  }
  try {
    for (std::size_t index = 0; index < entries_.size(); ++index) {
      auto& entry = entries_[index];
      if (entry.kind != kind || entry.pointer_token != pointer_token) continue;
      if ((kind == HostStubDescriptorKind::storage &&
           (entry.byte_len != byte_len || entry.alignment != alignment))) {
        return CaptureSerializationError::descriptor_conflict;
      }
      if (kind != HostStubDescriptorKind::storage &&
          std::find(entry.witnesses.begin(), entry.witnesses.end(), semantic_witness) ==
              entry.witnesses.end()) {
        entry.witnesses.emplace_back(semantic_witness);
      }
      stub_id = static_cast<std::uint32_t>(index);
      return CaptureSerializationError::ok;
    }
    if (entries_.size() >= 2'000'000) return CaptureSerializationError::limit_exceeded;
    Entry entry{};
    entry.kind = kind;
    entry.pointer_token = pointer_token;
    entry.byte_len = byte_len;
    entry.alignment = alignment;
    if (kind != HostStubDescriptorKind::storage) {
      entry.witnesses.emplace_back(semantic_witness);
    }
    entries_.push_back(std::move(entry));
    stub_id = static_cast<std::uint32_t>(entries_.size() - 1);
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError HostStubCatalog::finalize(
    std::vector<HostStubDescriptorProjection>& projection) const noexcept {
  try {
    std::vector<HostStubDescriptorProjection> value;
    value.reserve(entries_.size());
    for (std::size_t index = 0; index < entries_.size(); ++index) {
      const auto& entry = entries_[index];
      HostStubDescriptorProjection output{};
      output.stub_id = static_cast<std::uint32_t>(index);
      output.kind = entry.kind;
      output.pointer_token = entry.pointer_token;
      output.byte_len = entry.byte_len;
      output.alignment = entry.alignment;
      if (entry.kind != HostStubDescriptorKind::storage) {
        auto witnesses = entry.witnesses;
        std::sort(witnesses.begin(), witnesses.end());
        if (witnesses.empty() ||
            !sha256_witness_set(entry.kind, witnesses, output.semantic_sha256)) {
          return CaptureSerializationError::hash_failure;
        }
      }
      value.push_back(output);
    }
    projection = std::move(value);
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError HostStubCatalog::update_storage_descriptor(
    const std::uint32_t stub_id,
    const std::uint32_t byte_len,
    const std::uint32_t alignment) noexcept {
  if (stub_id >= entries_.size() ||
      entries_[stub_id].kind != HostStubDescriptorKind::storage ||
      byte_len == 0 || byte_len > 64u * 1024u * 1024u ||
      !valid_alignment(alignment)) {
    return CaptureSerializationError::invalid_argument;
  }
  auto& entry = entries_[stub_id];
  entry.byte_len = byte_len;
  entry.alignment = alignment;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::claim_registration(
    std::uint32_t& ordinal,
    std::uint32_t& registration_id) noexcept {
  if (registrations_ == std::numeric_limits<std::uint32_t>::max()) {
    return CaptureSerializationError::limit_exceeded;
  }
  ordinal = registrations_;
  registration_id = registrations_++;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::register_type(
    const std::uintptr_t capability,
    const std::uint32_t engine_id,
    std::uint32_t& trace_id) noexcept {
  if (capability == 0) return CaptureSerializationError::invalid_argument;
  if (std::any_of(types_.begin(), types_.end(), [&](const auto& value) {
        return value.capability == capability || value.engine_id == engine_id;
      })) {
    return CaptureSerializationError::duplicate_identity;
  }
  try {
    trace_id = next_type_id_++;
    types_.push_back({capability, trace_id, engine_id});
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError TraceIdCorrelation::register_function(
    const std::uint32_t engine_id,
    std::uint32_t& trace_id) noexcept {
  if (std::any_of(functions_.begin(), functions_.end(), [&](const auto& value) {
        return value.engine_id == engine_id;
      })) {
    return CaptureSerializationError::duplicate_identity;
  }
  try {
    trace_id = next_function_id_++;
    functions_.push_back({engine_id, trace_id});
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError TraceIdCorrelation::register_object_property(
    const std::uintptr_t owner,
    const std::uint32_t index,
    std::uint32_t& trace_id,
    std::uint32_t& owner_engine_id) noexcept {
  std::uint32_t owner_trace = 0;
  const auto resolved = type_ids(owner, owner_trace, owner_engine_id);
  if (resolved != CaptureSerializationError::ok) return resolved;
  (void)owner_trace;
  if (std::any_of(object_properties_.begin(), object_properties_.end(), [&](const auto& value) {
        return value.first == owner_engine_id && value.second == index;
      })) {
    return CaptureSerializationError::duplicate_identity;
  }
  try {
    trace_id = next_property_id_++;
    object_properties_.push_back({owner_engine_id, index, trace_id});
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError TraceIdCorrelation::register_global_property(
    const std::uint32_t index,
    std::uint32_t& trace_id) noexcept {
  if (std::any_of(global_properties_.begin(), global_properties_.end(), [&](const auto& value) {
        return value.engine_id == index;
      })) {
    return CaptureSerializationError::duplicate_identity;
  }
  try {
    trace_id = next_property_id_++;
    global_properties_.push_back({index, trace_id});
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError TraceIdCorrelation::type_ids(
    const std::uintptr_t capability,
    std::uint32_t& trace_id,
    std::uint32_t& engine_id) const noexcept {
  const auto found = std::find_if(types_.begin(), types_.end(), [&](const auto& value) {
    return value.capability == capability;
  });
  if (found == types_.end()) return CaptureSerializationError::unresolved_identity;
  trace_id = found->trace_id;
  engine_id = found->engine_id;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::trace_type_id_from_engine(
    const std::uint32_t engine_id,
    std::uint32_t& trace_id) const noexcept {
  const auto found = std::find_if(types_.begin(), types_.end(), [&](const auto& value) {
    return value.engine_id == engine_id;
  });
  if (found == types_.end()) return CaptureSerializationError::unresolved_identity;
  trace_id = found->trace_id;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::trace_function_id_from_engine(
    const std::uint32_t engine_id,
    std::uint32_t& trace_id) const noexcept {
  const auto found = std::find_if(functions_.begin(), functions_.end(), [&](const auto& value) {
    return value.engine_id == engine_id;
  });
  if (found == functions_.end()) return CaptureSerializationError::unresolved_identity;
  trace_id = found->trace_id;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::trace_object_property_id(
    const std::uint32_t owner_engine_id,
    const std::uint32_t index,
    std::uint32_t& trace_id) const noexcept {
  const auto found =
      std::find_if(object_properties_.begin(), object_properties_.end(), [&](const auto& value) {
        return value.first == owner_engine_id && value.second == index;
      });
  if (found == object_properties_.end()) return CaptureSerializationError::unresolved_identity;
  trace_id = found->trace_id;
  return CaptureSerializationError::ok;
}

CaptureSerializationError TraceIdCorrelation::trace_global_property_id(
    const std::uint32_t index,
    std::uint32_t& trace_id) const noexcept {
  const auto found = std::find_if(global_properties_.begin(), global_properties_.end(),
                                  [&](const auto& value) { return value.engine_id == index; });
  if (found == global_properties_.end()) return CaptureSerializationError::unresolved_identity;
  trace_id = found->trace_id;
  return CaptureSerializationError::ok;
}

CaptureSerializationError FinalStateJsonSequence::observe_registration(
    const RegistrationEntryJsonProjection& entry) noexcept {
  if (registrations_closed_ || registrations_ == std::numeric_limits<std::uint32_t>::max() ||
      entry.ordinal != registrations_ || entry.registration_id != registrations_) {
    return CaptureSerializationError::invalid_argument;
  }
  Kind kind{};
  bool required = true;
  switch (entry.kind) {
    case RegistrationEntryJsonKind::object_type:
      kind = Kind::object_type;
      break;
    case RegistrationEntryJsonKind::object_property:
      kind = Kind::object_property;
      break;
    case RegistrationEntryJsonKind::interface_method:
    case RegistrationEntryJsonKind::object_method:
    case RegistrationEntryJsonKind::object_behaviour:
    case RegistrationEntryJsonKind::global_function:
      kind = Kind::function;
      break;
    case RegistrationEntryJsonKind::global_property:
      kind = Kind::global_property;
      break;
    case RegistrationEntryJsonKind::interface:
    case RegistrationEntryJsonKind::enumeration:
    case RegistrationEntryJsonKind::enum_value:
    case RegistrationEntryJsonKind::funcdef:
    case RegistrationEntryJsonKind::type_alias:
    case RegistrationEntryJsonKind::string_factory:
    case RegistrationEntryJsonKind::default_array_type:
      required = false;
      break;
    default:
      return CaptureSerializationError::invalid_argument;
  }
  if (!required) {
    ++registrations_;
    return CaptureSerializationError::ok;
  }
  try {
    if (expected_.size() >= 2'000'000 ||
        std::find_if(expected_.begin(), expected_.end(), [&](const Expected& expected) {
          return expected.kind == kind && expected.trace_id == entry.trace_id;
        }) != expected_.end()) {
      return CaptureSerializationError::duplicate_identity;
    }
    expected_.push_back({kind, entry.trace_id});
    ++registrations_;
    return CaptureSerializationError::ok;
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError FinalStateJsonSequence::begin_final_state() noexcept {
  if (registrations_closed_) return CaptureSerializationError::invalid_argument;
  registrations_closed_ = true;
  return CaptureSerializationError::ok;
}

CaptureSerializationError FinalStateJsonSequence::validate_next(
    const Kind kind,
    const std::uint32_t trace_id) noexcept {
  if (!registrations_closed_ || next_ >= expected_.size() ||
      expected_[next_].kind != kind ||
      expected_[next_].trace_id != trace_id) {
    return CaptureSerializationError::unresolved_identity;
  }
  return CaptureSerializationError::ok;
}

void FinalStateJsonSequence::commit_next() noexcept { ++next_; }

CaptureSerializationError FinalStateJsonSequence::append(
    const ObjectTypeFinalState& state,
    std::string& json) noexcept {
  auto status = validate_next(Kind::object_type, state.type_id);
  if (status != CaptureSerializationError::ok) return status;
  status = serialize_final_state_json_v1(static_cast<std::uint32_t>(next_), state, json);
  if (status == CaptureSerializationError::ok) commit_next();
  return status;
}

CaptureSerializationError FinalStateJsonSequence::append(
    const ObjectPropertyFinalState& state,
    std::string& json) noexcept {
  auto status = validate_next(Kind::object_property, state.property_id);
  if (status != CaptureSerializationError::ok) return status;
  status = serialize_final_state_json_v1(static_cast<std::uint32_t>(next_), state, json);
  if (status == CaptureSerializationError::ok) commit_next();
  return status;
}

CaptureSerializationError FinalStateJsonSequence::append(
    const FunctionFinalState& state,
    std::string& json) noexcept {
  auto status = validate_next(Kind::function, state.function_id);
  if (status != CaptureSerializationError::ok) return status;
  status = serialize_final_state_json_v1(static_cast<std::uint32_t>(next_), state, json);
  if (status == CaptureSerializationError::ok) commit_next();
  return status;
}

CaptureSerializationError FinalStateJsonSequence::append(
    const GlobalPropertyFinalState& state,
    std::string& json) noexcept {
  auto status = validate_next(Kind::global_property, state.property_id);
  if (status != CaptureSerializationError::ok) return status;
  status = serialize_final_state_json_v1(static_cast<std::uint32_t>(next_), state, json);
  if (status == CaptureSerializationError::ok) commit_next();
  return status;
}

bool FinalStateJsonSequence::complete() const noexcept {
  return registrations_closed_ && next_ == expected_.size();
}

CaptureSerializationError serialize_registry_delta_json_v1(
    const std::uint32_t callback,
    const RegistrationEntryJsonProjection& entry,
    const RegistrationResultJsonProjection& result,
    std::string& output) noexcept {
  try {
    Json json;
    if (!json.raw(
            "{\"schema\":\"gore.as.capture.registry-delta\",\"schema_version\":1,"
            "\"bind_callback_ordinal\":") ||
        !json.u32(callback) || !json.raw(",\"entry\":") || !entry_json(json, entry) ||
        !json.raw(",\"result\":") || !result_json(json, entry.kind, result) ||
        !json.raw("}")) {
      return CaptureSerializationError::invalid_argument;
    }
    return finish_json(std::move(json), output);
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError serialize_registry_support_json_v1(
    const HostStubCatalog& catalog,
    std::string& output) noexcept {
  std::vector<HostStubDescriptorProjection> stubs;
  const auto status = catalog.finalize(stubs);
  if (status != CaptureSerializationError::ok) return status;
  try {
    Json json;
    if (!json.raw(
            "{\"schema\":\"gore.as.capture.registry-support\",\"schema_version\":1,"
            "\"host_stubs\":[")) {
      return CaptureSerializationError::limit_exceeded;
    }
    for (std::size_t index = 0; index < stubs.size(); ++index) {
      const auto& stub = stubs[index];
      if ((index != 0 && !json.raw(",")) || !json.raw("{\"stub_id\":") ||
          !json.u32(stub.stub_id) ||
          !json.raw(",\"purpose\":\"compile_only_never_invoke\",\"descriptor\":{")) {
        return CaptureSerializationError::limit_exceeded;
      }
      if (stub.kind == HostStubDescriptorKind::callable) {
        if (!json.raw("\"kind\":\"callable\",\"signature_sha256\":") ||
            !hex_digest(json, stub.semantic_sha256)) {
          return CaptureSerializationError::limit_exceeded;
        }
      } else if (stub.kind == HostStubDescriptorKind::object) {
        if (!json.raw("\"kind\":\"object\",\"interface_sha256\":") ||
            !hex_digest(json, stub.semantic_sha256)) {
          return CaptureSerializationError::limit_exceeded;
        }
      } else if (!json.raw("\"kind\":\"storage\",\"byte_len\":") ||
                 !json.u32(stub.byte_len) || !json.raw(",\"alignment\":") ||
                 !json.u32(stub.alignment)) {
        return CaptureSerializationError::limit_exceeded;
      }
      if (!json.raw("}}")) return CaptureSerializationError::limit_exceeded;
    }
    if (!json.raw("],\"host_stub_pointers\":[")) {
      return CaptureSerializationError::limit_exceeded;
    }
    for (std::size_t index = 0; index < stubs.size(); ++index) {
      if ((index != 0 && !json.raw(",")) || !json.raw("{\"stub_id\":") ||
          !json.u32(stubs[index].stub_id) || !json.raw(",\"pointer_token\":") ||
          !json.u32(stubs[index].pointer_token) || !json.raw("}")) {
        return CaptureSerializationError::limit_exceeded;
      }
    }
    constexpr std::array primitive_names{
        "bool", "int8", "int16", "int32", "int64", "uint8",
        "uint16", "uint32", "uint64", "float32", "float64"};
    constexpr std::array<std::uint32_t, 11> sizes{1, 1, 2, 4, 8, 1, 2, 4, 8, 4, 8};
    if (!json.raw("],\"primitive_operations\":[")) {
      return CaptureSerializationError::limit_exceeded;
    }
    for (std::size_t index = 0; index < primitive_names.size(); ++index) {
      FixedTypeOperationsProjection operations{
          true, false, false, true, true, false, true, false, true, false, true, true,
          sizes[index], sizes[index], false};
      if ((index != 0 && !json.raw(",")) || !json.raw("{\"ordinal\":") ||
          !json.u32(static_cast<std::uint32_t>(index)) || !json.raw(",\"primitive\":\"") ||
          !json.raw(primitive_names[index]) || !json.raw("\",\"operations\":") ||
          !fixed_operations_json(json, operations) || !json.raw("}")) {
        return CaptureSerializationError::limit_exceeded;
      }
    }
    FixedTypeOperationsProjection dynamic{
        true, false, false, true, true, true, true, true, true, true, true, false,
        16, 8, false};
    if (!json.raw("],\"dynamic_script_operations\":{\"delegate\":") ||
        !fixed_operations_json(json, dynamic) || !json.raw(",\"multicast_delegate\":") ||
        !fixed_operations_json(json, dynamic) || !json.raw("}}")) {
      return CaptureSerializationError::limit_exceeded;
    }
    return finish_json(std::move(json), output);
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError serialize_final_state_json_v1(
    const std::uint32_t ordinal,
    const ObjectTypeFinalState& value,
    std::string& output) noexcept {
  try {
    Json json;
    if (!valid_alignment(value.alignment) ||
        (!value.has_base_type && value.base_type_id != 0) ||
        (!value.has_shadow_type && value.shadow_type_id != 0) ||
        value.interface_type_ids.size() != value.interface_vft_offsets.size() ||
        !final_prefix(json, ordinal, "object_type") || !json.raw("\"type_id\":") ||
        !json.u32(value.type_id) || !json.raw(",\"byte_size\":") ||
        !json.u32(value.byte_size) || !json.raw(",\"alignment\":") ||
        !json.u32(value.alignment) || !json.raw(",\"flags\":") || !json.u32(value.flags) ||
        !json.raw(",\"base_type_id\":") ||
        !(value.has_base_type ? json.u32(value.base_type_id) : json.raw("null")) ||
        !json.raw(",\"shadow_type_id\":") ||
        !(value.has_shadow_type ? json.u32(value.shadow_type_id) : json.raw("null")) ||
        !json.raw(",\"interface_type_ids\":") || !ids(json, value.interface_type_ids) ||
        !json.raw(",\"interface_vft_offsets\":") ||
        !ids(json, value.interface_vft_offsets) ||
        !json.raw(",\"has_implicit_constructors\":") ||
        !json.boolean(value.has_implicit_constructors) ||
        !json.raw(",\"accepts_value_subtype\":") ||
        !json.boolean(value.accepts_value_subtype) ||
        !json.raw(",\"accepts_reference_subtype\":") ||
        !json.boolean(value.accepts_reference_subtype) ||
        !json.raw(",\"is_invalid_generated_type\":") ||
        !json.boolean(value.is_invalid_generated_type) || !json.raw("}}")) {
      return CaptureSerializationError::invalid_argument;
    }
    return finish_json(std::move(json), output);
  } catch (...) {
    return CaptureSerializationError::limit_exceeded;
  }
}

CaptureSerializationError serialize_final_state_json_v1(
    const std::uint32_t ordinal,
    const ObjectPropertyFinalState& value,
    std::string& output) noexcept {
  Json json;
  if (value.exposed_type > 0xff || !final_prefix(json, ordinal, "object_property") ||
      !json.raw("\"property_id\":") || !json.u32(value.property_id) ||
      !json.raw(",\"byte_offset\":") || !json.u32(value.byte_offset) ||
      !json.raw(",\"access_mask\":") || !json.u32(value.access_mask) ||
      !json.raw(",\"composite_offset\":") || !json.u32(value.composite_offset) ||
      !json.raw(",\"is_composite_indirect\":") ||
      !json.boolean(value.is_composite_indirect) || !json.raw(",\"is_private\":") ||
      !json.boolean(value.is_private) || !json.raw(",\"is_protected\":") ||
      !json.boolean(value.is_protected) || !json.raw(",\"is_app_bind_property\":") ||
      !json.boolean(value.is_app_bind_property) || !json.raw(",\"exposed_type\":") ||
      !json.u32(value.exposed_type) || !json.raw("}}")) {
    return CaptureSerializationError::invalid_argument;
  }
  return finish_json(std::move(json), output);
}

CaptureSerializationError serialize_final_state_json_v1(
    const std::uint32_t ordinal,
    const FunctionFinalState& value,
    std::string& output) noexcept {
  constexpr std::array modes{
      "compile_calls", "compile_out_entirely", "replace_with_first_param",
      "compile_out_as_method_chain"};
  constexpr std::array metadata{"none", "script_function", "script_object_type"};
  Json json;
  if (value.exposed_type > 0xff || value.compile_out_mode >= modes.size() ||
      value.first_param_metadata >= metadata.size() ||
      value.has_hidden_argument != !value.hidden_argument_default.empty() ||
      (!value.has_hidden_argument && value.hidden_argument_index != 0) ||
      (!value.has_output_type_argument && value.output_type_argument_index != 0) ||
      !valid_utf8(value.hidden_argument_default) || !final_prefix(json, ordinal, "function") ||
      !json.raw("\"function_id\":") || !json.u32(value.function_id) ||
      !json.raw(",\"trait_bits\":") || !json.u32(value.trait_bits) ||
      !json.raw(",\"exposed_type\":") || !json.u32(value.exposed_type) ||
      !json.raw(",\"hidden_argument_index\":") ||
      !(value.has_hidden_argument ? json.u32(value.hidden_argument_index) : json.raw("null")) ||
      !json.raw(",\"hidden_argument_default\":") ||
      !(value.has_hidden_argument ? json.string(value.hidden_argument_default)
                                  : json.raw("null")) ||
      !json.raw(",\"determines_output_type_argument_index\":") ||
      !(value.has_output_type_argument ? json.u32(value.output_type_argument_index)
                                       : json.raw("null")) ||
      !json.raw(",\"compile_out_mode\":\"") || !json.raw(modes[value.compile_out_mode]) ||
      !json.raw("\",\"first_param_metadata\":\"") ||
      !json.raw(metadata[value.first_param_metadata]) || !json.raw("\"}}")) {
    return CaptureSerializationError::invalid_argument;
  }
  return finish_json(std::move(json), output);
}

CaptureSerializationError serialize_final_state_json_v1(
    const std::uint32_t ordinal,
    const GlobalPropertyFinalState& value,
    std::string& output) noexcept {
  Json json;
  if ((!value.is_pure_constant && value.pure_constant_value != 0) ||
      !final_prefix(json, ordinal, "global_property") ||
      !json.raw("\"property_id\":") || !json.u32(value.property_id) ||
      !json.raw(",\"is_pure_constant\":") || !json.boolean(value.is_pure_constant) ||
      !json.raw(",\"pure_constant_value\":") ||
      !(value.is_pure_constant ? json.u64(value.pure_constant_value) : json.raw("null")) ||
      !json.raw("}}")) {
    return CaptureSerializationError::invalid_argument;
  }
  return finish_json(std::move(json), output);
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_capture_serializer_selftest_v1() noexcept {
  TraceIdCorrelation identities;
  std::uint32_t ordinal = 99;
  std::uint32_t registration = 99;
  std::uint32_t type_id = 99;
  std::uint32_t function_id = 99;
  std::uint32_t property_id = 99;
  std::uint32_t owner_engine = 0;
  if (identities.claim_registration(ordinal, registration) != CaptureSerializationError::ok ||
      ordinal != 0 || registration != 0 ||
      identities.register_type(0x1000, 81, type_id) != CaptureSerializationError::ok ||
      type_id != 0 ||
      identities.register_function(91, function_id) != CaptureSerializationError::ok ||
      function_id != 0 ||
      identities.register_object_property(0x1000, 3, property_id, owner_engine) !=
          CaptureSerializationError::ok ||
      property_id != 0 || owner_engine != 81 ||
      identities.register_global_property(7, property_id) != CaptureSerializationError::ok ||
      property_id != 1 || identities.register_type(0x1000, 82, type_id) !=
                              CaptureSerializationError::duplicate_identity ||
      identities.trace_type_id_from_engine(81, type_id) != CaptureSerializationError::ok ||
      type_id != 0 || identities.trace_function_id_from_engine(91, function_id) !=
                          CaptureSerializationError::ok ||
      function_id != 0 ||
      identities.trace_object_property_id(81, 3, property_id) !=
          CaptureSerializationError::ok ||
      property_id != 0 || identities.trace_global_property_id(7, property_id) !=
                              CaptureSerializationError::ok ||
      property_id != 1 || identities.trace_global_property_id(8, property_id) !=
                              CaptureSerializationError::unresolved_identity) {
    return false;
  }

  HostStubCatalog catalog;
  std::uint32_t callable = 99;
  std::uint32_t storage = 99;
  std::uint32_t object = 99;
  RegistrationEntryJsonProjection callable_entry{};
  callable_entry.kind = RegistrationEntryJsonKind::global_function;
  callable_entry.context.name_space = "Fixture";
  callable_entry.context.access_mask = 7;
  callable_entry.declaration = "void f(int)";
  callable_entry.call_convention = "cdecl";
  RegistrationStubCapabilities callable_capabilities{};
  callable_capabilities.has_callable = true;
  callable_capabilities.callable_pointer_token = 4;
  RegistrationEntryJsonProjection storage_entry{};
  storage_entry.kind = RegistrationEntryJsonKind::global_property;
  RegistrationStubCapabilities storage_capabilities{};
  storage_capabilities.has_storage = true;
  storage_capabilities.storage_pointer_token = 5;
  storage_capabilities.storage_byte_len = 16;
  storage_capabilities.storage_alignment = 8;
  RegistrationEntryJsonProjection factory_entry{};
  factory_entry.kind = RegistrationEntryJsonKind::string_factory;
  factory_entry.declaration = "string";
  RegistrationStubCapabilities factory_capabilities{};
  factory_capabilities.has_factory_object = true;
  factory_capabilities.factory_object_pointer_token = 4;
  const auto callable_first =
      catalog.derive_registration_stubs(callable_entry, callable_capabilities);
  callable_entry.declaration = "void f(float)";
  const auto callable_second =
      catalog.derive_registration_stubs(callable_entry, callable_capabilities);
  const auto storage_first =
      catalog.derive_registration_stubs(storage_entry, storage_capabilities);
  const auto factory_first =
      catalog.derive_registration_stubs(factory_entry, factory_capabilities);
  storage_capabilities.storage_byte_len = 8;
  const auto storage_conflict =
      catalog.derive_registration_stubs(storage_entry, storage_capabilities);
  if (callable_first != CaptureSerializationError::ok ||
      callable_entry.callable_stub_id != 0 ||
      callable_second != CaptureSerializationError::ok ||
      callable_entry.callable_stub_id != 0 ||
      storage_first != CaptureSerializationError::ok ||
      storage_entry.storage_stub_id != 1 ||
      factory_first != CaptureSerializationError::ok ||
      factory_entry.factory_object_stub_id != 2 ||
      storage_conflict != CaptureSerializationError::descriptor_conflict) {
    return false;
  }
  callable = callable_entry.callable_stub_id;
  storage = storage_entry.storage_stub_id;
  object = factory_entry.factory_object_stub_id;

  HostStubCatalog auxiliary_catalog;
  RegistrationEntryJsonProjection auxiliary_entry{};
  auxiliary_entry.kind = RegistrationEntryJsonKind::global_function;
  auxiliary_entry.context.name_space = "Fixture";
  auxiliary_entry.declaration = "void MethodLike()";
  auxiliary_entry.call_convention = "thiscall_as_global";
  RegistrationStubCapabilities auxiliary_capabilities{};
  auxiliary_capabilities.has_callable = true;
  auxiliary_capabilities.callable_pointer_token = 6;
  auxiliary_capabilities.has_auxiliary_object = true;
  auxiliary_capabilities.auxiliary_object_pointer_token = 7;
  if (auxiliary_catalog.derive_registration_stubs(auxiliary_entry, auxiliary_capabilities) !=
          CaptureSerializationError::ok ||
      auxiliary_entry.callable_stub_id != 0 ||
      !auxiliary_entry.has_auxiliary_object_stub ||
      auxiliary_entry.auxiliary_object_stub_id != 1) {
    return false;
  }
  auxiliary_entry.call_convention = "cdecl";
  if (auxiliary_catalog.derive_registration_stubs(auxiliary_entry, auxiliary_capabilities) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }
  std::vector<HostStubDescriptorProjection> auxiliary_projection;
  if (auxiliary_catalog.finalize(auxiliary_projection) != CaptureSerializationError::ok ||
      auxiliary_projection.size() != 2) {
    return false;
  }
  std::string support;
  if (serialize_registry_support_json_v1(catalog, support) !=
          CaptureSerializationError::ok ||
      support.find("\"stub_id\":2") == std::string::npos ||
      support.find("\"primitive\":\"float64\"") == std::string::npos ||
      support.find("\"need_construct\":true") == std::string::npos) {
    return false;
  }

  RegistrationEntryJsonProjection entry{};
  entry.kind = RegistrationEntryJsonKind::global_function;
  entry.ordinal = 0;
  entry.registration_id = 0;
  entry.context.name_space = "Fixture\\\"Namespace";
  entry.context.access_mask = 7;
  entry.trace_id = 0;
  entry.declaration = "void Fixture(int)";
  entry.call_convention = "cdecl";
  entry.callable_stub_id = callable;
  RegistrationResultJsonProjection result{};
  if (project_registration_post_result_v23300(
          1, 91, result.post_result) != FinalStateError::ok) {
    return false;
  }
  std::string delta;
  std::string delta_replay;
  if (serialize_registry_delta_json_v1(3, entry, result, delta) !=
          CaptureSerializationError::ok ||
      serialize_registry_delta_json_v1(3, entry, result, delta_replay) !=
          CaptureSerializationError::ok ||
      delta != delta_replay ||
      delta.find("\"bind_callback_ordinal\":3") == std::string::npos ||
      delta.find("\"kind\":\"global_function\"") == std::string::npos ||
      delta.find("\"engine_function_id\":91") == std::string::npos ||
      delta.find("Fixture\\\\\\\"Namespace") == std::string::npos) {
    return false;
  }
  result.has_owner_engine_type_id = true;
  if (serialize_registry_delta_json_v1(3, entry, result, delta) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }

  struct RegistrationCase final {
    RegistrationEntryJsonKind kind;
    std::uint32_t hook_kind;
    const char* json_kind;
    bool has_owner;
  };
  constexpr std::array registration_cases{
      RegistrationCase{RegistrationEntryJsonKind::object_type, 3, "object_type", false},
      RegistrationCase{RegistrationEntryJsonKind::interface, 7, "interface", false},
      RegistrationCase{RegistrationEntryJsonKind::interface_method, 8, "interface_method", true},
      RegistrationCase{RegistrationEntryJsonKind::object_property, 4, "object_property", true},
      RegistrationCase{RegistrationEntryJsonKind::object_method, 5, "object_method", true},
      RegistrationCase{
          RegistrationEntryJsonKind::object_behaviour, 6, "object_behaviour", true},
      RegistrationCase{RegistrationEntryJsonKind::global_property, 2, "global_property", false},
      RegistrationCase{RegistrationEntryJsonKind::global_function, 1, "global_function", false},
      RegistrationCase{RegistrationEntryJsonKind::enumeration, 11, "enum", false},
      RegistrationCase{RegistrationEntryJsonKind::enum_value, 12, "enum_value", true},
      RegistrationCase{RegistrationEntryJsonKind::funcdef, 13, "funcdef", false},
      RegistrationCase{RegistrationEntryJsonKind::type_alias, 14, "typedef", false},
      RegistrationCase{RegistrationEntryJsonKind::string_factory, 9, "string_factory", false},
      RegistrationCase{
          RegistrationEntryJsonKind::default_array_type, 10, "default_array_type", false},
  };
  FinalStateJsonSequence final_sequence;
  for (std::size_t index = 0; index < registration_cases.size(); ++index) {
    const auto& fixture = registration_cases[index];
    RegistrationEntryJsonProjection projected{};
    projected.kind = fixture.kind;
    projected.ordinal = static_cast<std::uint32_t>(index);
    projected.registration_id = static_cast<std::uint32_t>(index);
    projected.context.name_space = "Fixture";
    projected.context.access_mask = 7;
    constexpr std::array<std::uint32_t, 14> trace_ids{
        0, 1, 0, 0, 1, 2, 1, 3, 2, 0, 3, 4, 0, 0};
    projected.trace_id = trace_ids[index];
    projected.owner_trace_type_id = 0;
    projected.declaration = "void Entry()";
    projected.name = "Entry";
    projected.target_declaration = "uint32";
    projected.byte_size = 16;
    projected.alignment = 8;
    projected.type_operations = TypeOperationsJsonKind::unavailable;
    projected.call_convention = "cdecl";
    projected.callable_stub_id = callable;
    projected.behaviour = "construct";
    projected.storage_stub_id = storage;
    projected.factory_object_stub_id = object;

    RegistrationResultJsonProjection projected_result{};
    projected_result.has_owner_engine_type_id = fixture.has_owner;
    projected_result.owner_engine_type_id = 81;
    const std::int32_t native_result =
        fixture.hook_kind == 9 || fixture.hook_kind == 10
            ? 0
            : static_cast<std::int32_t>(100 + fixture.hook_kind);
    if (final_sequence.observe_registration(projected) != CaptureSerializationError::ok ||
        project_registration_post_result_v23300(
            fixture.hook_kind, native_result, projected_result.post_result) !=
            FinalStateError::ok ||
        serialize_registry_delta_json_v1(3, projected, projected_result, delta) !=
            CaptureSerializationError::ok ||
        delta.find(std::string{"\"kind\":\""} + fixture.json_kind + "\"") ==
            std::string::npos) {
      return false;
    }
  }
  if (final_sequence.complete() ||
      final_sequence.begin_final_state() != CaptureSerializationError::ok ||
      final_sequence.begin_final_state() != CaptureSerializationError::invalid_argument) {
    return false;
  }

  entry.call_convention = "unknown_abi";
  result.has_owner_engine_type_id = false;
  if (serialize_registry_delta_json_v1(3, entry, result, delta) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }
  entry.call_convention = "cdecl";
  entry.kind = RegistrationEntryJsonKind::object_behaviour;
  entry.behaviour = "template_callback";
  entry.owner_trace_type_id = 0;
  result.has_owner_engine_type_id = true;
  result.owner_engine_type_id = 81;
  if (project_registration_post_result_v23300(
          6, 101, result.post_result) != FinalStateError::ok ||
      serialize_registry_delta_json_v1(3, entry, result, delta) !=
          CaptureSerializationError::invalid_argument) {
    return false;
  }
  entry.has_template_validation_adapter = true;
  entry.template_validation_adapter = "unknown_adapter";
  if (serialize_registry_delta_json_v1(3, entry, result, delta) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }

  ObjectTypeFinalState state{};
  state.type_id = 0;
  state.byte_size = 32;
  state.alignment = 8;
  state.flags = 4;
  state.has_base_type = true;
  state.base_type_id = 1;
  state.interface_type_ids = {2};
  state.interface_vft_offsets = {16};
  state.has_implicit_constructors = true;
  state.accepts_value_subtype = true;
  std::string first;
  std::string second;
  if (serialize_final_state_json_v1(0, state, first) != CaptureSerializationError::ok ||
      serialize_final_state_json_v1(0, state, second) != CaptureSerializationError::ok ||
      first != second || first.find("\"shadow_type_id\":null") == std::string::npos ||
      first.find("\"interface_vft_offsets\":[16]") == std::string::npos) {
    return false;
  }
  state.interface_vft_offsets.clear();
  if (serialize_final_state_json_v1(0, state, first) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }

  FunctionFinalState function{};
  function.function_id = 0;
  function.exposed_type = 2;
  function.has_hidden_argument = true;
  function.hidden_argument_index = 1;
  function.hidden_argument_default = "World\nContext";
  function.compile_out_mode = 3;
  function.first_param_metadata = 2;
  if (serialize_final_state_json_v1(1, function, first) != CaptureSerializationError::ok ||
      first.find("\"compile_out_mode\":\"compile_out_as_method_chain\"") ==
          std::string::npos ||
      first.find("World\\nContext") == std::string::npos) {
    return false;
  }
  function.compile_out_mode = 4;
  if (serialize_final_state_json_v1(1, function, first) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }

  ObjectPropertyFinalState object_property{};
  object_property.property_id = 1;
  object_property.byte_offset = 8;
  object_property.access_mask = 7;
  object_property.composite_offset = 16;
  object_property.is_composite_indirect = true;
  object_property.is_private = true;
  object_property.is_app_bind_property = true;
  object_property.exposed_type = 4;
  if (serialize_final_state_json_v1(2, object_property, first) !=
          CaptureSerializationError::ok ||
      first.find("\"kind\":\"object_property\"") == std::string::npos ||
      first.find("\"is_app_bind_property\":true") == std::string::npos) {
    return false;
  }
  object_property.exposed_type = 256;
  if (serialize_final_state_json_v1(2, object_property, first) !=
      CaptureSerializationError::invalid_argument) {
    return false;
  }

  GlobalPropertyFinalState global_property{};
  global_property.property_id = 2;
  global_property.is_pure_constant = true;
  global_property.pure_constant_value = 0xfedcba9876543210ull;
  if (serialize_final_state_json_v1(3, global_property, first) !=
          CaptureSerializationError::ok ||
      serialize_final_state_json_v1(3, global_property, second) !=
          CaptureSerializationError::ok ||
      first != second ||
      first.find("\"pure_constant_value\":18364758544493064720") ==
          std::string::npos) {
    return false;
  }

  ObjectTypeFinalState ordered_type{};
  ordered_type.type_id = 0;
  ordered_type.byte_size = 16;
  ordered_type.alignment = 8;
  FunctionFinalState ordered_function{};
  ordered_function.function_id = 0;
  ObjectPropertyFinalState ordered_property{};
  ordered_property.property_id = 0;
  GlobalPropertyFinalState ordered_global{};
  ordered_global.property_id = 1;
  const auto wrong_order = final_sequence.append(ordered_function, first);
  const auto type_status = final_sequence.append(ordered_type, first);
  const auto interface_method_status = final_sequence.append(ordered_function, first);
  const auto property_status = final_sequence.append(ordered_property, first);
  ordered_function.function_id = 1;
  const auto object_method_status = final_sequence.append(ordered_function, first);
  ordered_function.function_id = 2;
  const auto behaviour_status = final_sequence.append(ordered_function, first);
  const auto global_property_status = final_sequence.append(ordered_global, first);
  ordered_function.function_id = 3;
  const auto global_function_status = final_sequence.append(ordered_function, first);
  if (wrong_order != CaptureSerializationError::unresolved_identity ||
      type_status != CaptureSerializationError::ok ||
      interface_method_status != CaptureSerializationError::ok ||
      property_status != CaptureSerializationError::ok ||
      object_method_status != CaptureSerializationError::ok ||
      behaviour_status != CaptureSerializationError::ok ||
      global_property_status != CaptureSerializationError::ok ||
      global_function_status != CaptureSerializationError::ok || !final_sequence.complete() ||
      final_sequence.append(ordered_function, first) !=
          CaptureSerializationError::unresolved_identity) {
    return false;
  }
  return true;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
