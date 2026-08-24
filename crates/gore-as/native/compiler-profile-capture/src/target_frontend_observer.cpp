#include "target_frontend_observer.hpp"

#include "gore_as_capture/format.hpp"

#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <set>
#include <string_view>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::size_t kMaximumTextBytes = 16u * 1024u * 1024u;
constexpr std::size_t kMaximumItems = 1'000'000u;
constexpr std::size_t kMaximumConfigurationItems = 16'384u;
constexpr char kGraphInputDomain[] = "gore-as-external-hook-graph-input-v1\0";
constexpr char kGraphOutputDomain[] = "gore-as-external-hook-graph-output-v1\0";
constexpr char kPreprocessorDomain[] = "gore-as-preprocessor-config-v1\0";
constexpr char kClassGeneratorDomain[] = "gore-as-class-generator-config-v1\0";
constexpr char kCompilerOptionsDomain[] = "gore-as-compiler-options-v1\0";
constexpr char kFrontendConfigSetDomain[] = "gore-as-captured-frontend-config-set-v1\0";

bool valid_utf8(
    const std::string_view value,
    const bool allow_empty = true,
    const std::size_t maximum_bytes = kMaximumTextBytes,
    const bool forbid_controls = false) noexcept {
  if ((!allow_empty && value.empty()) || value.size() > maximum_bytes ||
      value.find('\0') != std::string_view::npos) {
    return false;
  }
  std::size_t cursor = 0;
  while (cursor < value.size()) {
    const auto lead = static_cast<std::uint8_t>(value[cursor]);
    if (lead < 0x80) {
      if (forbid_controls && (lead < 0x20 || lead == 0x7f)) return false;
      ++cursor;
      continue;
    }
    std::size_t continuation = 0;
    std::uint32_t scalar = 0;
    std::uint32_t minimum = 0;
    if ((lead & 0xe0u) == 0xc0u) {
      continuation = 1;
      scalar = lead & 0x1fu;
      minimum = 0x80;
    } else if ((lead & 0xf0u) == 0xe0u) {
      continuation = 2;
      scalar = lead & 0x0fu;
      minimum = 0x800;
    } else if ((lead & 0xf8u) == 0xf0u) {
      continuation = 3;
      scalar = lead & 0x07u;
      minimum = 0x10000;
    } else {
      return false;
    }
    if (continuation > value.size() - cursor - 1) return false;
    for (std::size_t index = 1; index <= continuation; ++index) {
      const auto byte = static_cast<std::uint8_t>(value[cursor + index]);
      if ((byte & 0xc0u) != 0x80u) return false;
      scalar = (scalar << 6) | (byte & 0x3fu);
    }
    if (scalar < minimum || scalar > 0x10ffff ||
        (scalar >= 0xd800 && scalar <= 0xdfff)) {
      return false;
    }
    if (forbid_controls && scalar >= 0x80 && scalar <= 0x9f) return false;
    cursor += continuation + 1;
  }
  return true;
}

class Sha256 final {
 public:
  Sha256() noexcept {
    DWORD output = 0;
    if (BCryptOpenAlgorithmProvider(&algorithm_, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0 ||
        BCryptGetProperty(
            algorithm_,
            BCRYPT_OBJECT_LENGTH,
            reinterpret_cast<PUCHAR>(&object_bytes_),
            sizeof(object_bytes_),
            &output,
            0) < 0 ||
        output != sizeof(object_bytes_) || object_bytes_ == 0 || object_bytes_ > 64 * 1024) {
      close();
      return;
    }
    try {
      object_.resize(object_bytes_);
    } catch (...) {
      close();
      return;
    }
    if (BCryptCreateHash(
            algorithm_, &hash_, object_.data(), object_bytes_, nullptr, 0, 0) < 0) {
      close();
    }
  }
  ~Sha256() { close(); }
  Sha256(const Sha256&) = delete;
  Sha256& operator=(const Sha256&) = delete;

  bool append(const void* const bytes, const std::size_t size) noexcept {
    if (hash_ == nullptr || (size != 0 && bytes == nullptr) ||
        size > std::numeric_limits<ULONG>::max()) {
      return false;
    }
    return size == 0 || BCryptHashData(
                            hash_,
                            const_cast<PUCHAR>(static_cast<const UCHAR*>(bytes)),
                            static_cast<ULONG>(size),
                            0) >= 0;
  }
  bool append(const std::string_view value) noexcept {
    return append(value.data(), value.size());
  }
  bool finish(FrontendDigest& digest) noexcept {
    if (hash_ == nullptr ||
        BCryptFinishHash(hash_, digest.data(), static_cast<ULONG>(digest.size()), 0) < 0) {
      return false;
    }
    (void)BCryptDestroyHash(hash_);
    hash_ = nullptr;
    return true;
  }

 private:
  void close() noexcept {
    if (hash_ != nullptr) (void)BCryptDestroyHash(hash_);
    if (algorithm_ != nullptr) (void)BCryptCloseAlgorithmProvider(algorithm_, 0);
    hash_ = nullptr;
    algorithm_ = nullptr;
  }
  BCRYPT_ALG_HANDLE algorithm_{};
  BCRYPT_HASH_HANDLE hash_{};
  DWORD object_bytes_{};
  std::vector<std::uint8_t> object_;
};

bool append_u64(Sha256& hash, const std::uint64_t value) noexcept {
  std::array<std::uint8_t, 8> bytes{};
  for (std::size_t index = 0; index < bytes.size(); ++index) {
    bytes[index] = static_cast<std::uint8_t>(value >> (index * 8));
  }
  return hash.append(bytes.data(), bytes.size());
}

bool append_field(Sha256& hash, const std::string_view value) noexcept {
  return append_u64(hash, value.size()) && hash.append(value);
}

bool sha256_bytes(const std::string_view value, FrontendDigest& digest) noexcept {
  Sha256 hash;
  return hash.append(value) && hash.finish(digest);
}

bool graph_input_digest(
    const std::vector<FrontendGraphModule>& modules,
    FrontendDigest& digest) noexcept {
  Sha256 hash;
  if (!hash.append(kGraphInputDomain, sizeof(kGraphInputDomain) - 1) ||
      !append_u64(hash, modules.size())) {
    return false;
  }
  for (const auto& module : modules) {
    if (!append_field(hash, module.module_name) || !append_u64(hash, module.sections.size())) {
      return false;
    }
    for (const auto& section : module.sections) {
      if (!append_field(hash, section.relative_path) || !append_field(hash, section.code)) {
        return false;
      }
    }
  }
  return hash.finish(digest);
}

bool graph_output_digest(
    const FrontendDigest& input,
    const std::vector<FrontendGraphModule>& modules,
    FrontendDigest& digest) noexcept {
  Sha256 hash;
  if (!hash.append(kGraphOutputDomain, sizeof(kGraphOutputDomain) - 1) ||
      !hash.append(input.data(), input.size()) || !append_u64(hash, modules.size())) {
    return false;
  }
  for (const auto& module : modules) {
    if (!append_field(hash, module.module_name) ||
        !append_field(hash, module.generated_declarations)) {
      return false;
    }
  }
  return hash.finish(digest);
}

bool valid_module_graph(const std::vector<FrontendGraphModule>& modules) noexcept {
  if (modules.size() > 4096) return false;
  std::set<std::string_view> names;
  std::size_t bytes = 0;
  for (const auto& module : modules) {
    if (!valid_utf8(module.module_name, false, 4096) ||
        !names.insert(module.module_name).second ||
        module.sections.size() > 4096 ||
        !valid_utf8(module.generated_declarations)) {
      return false;
    }
    if (bytes > kMaximumTextBytes - module.generated_declarations.size()) return false;
    bytes += module.generated_declarations.size();
    for (const auto& section : module.sections) {
      if (!valid_utf8(section.relative_path, false, 4096) || !valid_utf8(section.code)) {
        return false;
      }
    }
  }
  return true;
}

bool same_graph_input(
    const std::vector<FrontendGraphModule>& left,
    const std::vector<FrontendGraphModule>& right) noexcept {
  if (left.size() != right.size()) return false;
  for (std::size_t index = 0; index < left.size(); ++index) {
    if (left[index].module_name != right[index].module_name ||
        left[index].sections.size() != right[index].sections.size()) {
      return false;
    }
    for (std::size_t section = 0; section < left[index].sections.size(); ++section) {
      if (left[index].sections[section].relative_path !=
              right[index].sections[section].relative_path ||
          left[index].sections[section].code != right[index].sections[section].code) {
        return false;
      }
    }
  }
  return true;
}

bool valid_class_captures(const std::vector<FrontendClassCapture>& captures) noexcept {
  if (captures.size() > kMaximumItems) return false;
  std::set<std::array<std::string_view, 3>> identities;
  std::size_t generated_bytes = 0;
  for (std::size_t index = 0; index < captures.size(); ++index) {
    const auto& capture = captures[index];
    if (capture.ordinal != index ||
        !valid_utf8(capture.module_name, false, 4096) ||
        !valid_utf8(capture.name_space, true, 4096) ||
        !valid_utf8(capture.class_name, false, 4096) ||
        !valid_utf8(capture.compose_onto_class, true, 4096) ||
        !valid_utf8(capture.generated_statics) ||
        generated_bytes > kMaximumTextBytes - capture.generated_statics.size() ||
        !identities.insert(
            {capture.module_name, capture.name_space, capture.class_name}).second) {
      return false;
    }
    generated_bytes += capture.generated_statics.size();
    FrontendDigest actual{};
    if (!sha256_bytes(capture.generated_statics, actual) ||
        actual != capture.output_generated_statics_sha256) {
      return false;
    }
  }
  return true;
}

bool valid_graph_captures(const std::vector<FrontendGraphCapture>& captures) noexcept {
  if (captures.size() > 4096) return false;
  std::set<FrontendDigest> inputs;
  for (std::size_t index = 0; index < captures.size(); ++index) {
    const auto& capture = captures[index];
    if (capture.ordinal != index || capture.modules.size() > 4096 ||
        !inputs.insert(capture.input_graph_sha256).second) {
      return false;
    }
    std::set<std::string_view> names;
    std::size_t generated_bytes = 0;
    for (const auto& module : capture.modules) {
      if (!valid_utf8(module.module_name, false, 4096) ||
          !valid_utf8(module.generated_declarations) ||
          !names.insert(module.module_name).second ||
          generated_bytes > kMaximumTextBytes - module.generated_declarations.size()) {
        return false;
      }
      generated_bytes += module.generated_declarations.size();
    }
    FrontendDigest actual{};
    if (!graph_output_digest(capture.input_graph_sha256, capture.modules, actual) ||
        actual != capture.output_graph_sha256) {
      return false;
    }
  }
  return true;
}

class Json final {
 public:
  bool raw(const std::string_view value) {
    if (value.size() > 4u * 1024u * 1024u -
                           std::min<std::size_t>(output_.size(), 4u * 1024u * 1024u)) {
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
            const std::array<char, 6> escaped{
                '\\', 'u', '0', '0', hex[byte >> 4], hex[byte & 0x0f]};
            if (!raw(std::string_view(escaped.data(), escaped.size()))) return false;
          } else if (!raw(std::string_view(
                         reinterpret_cast<const char*>(&byte), 1))) {
            return false;
          }
      }
    }
    return raw("\"");
  }
  bool boolean(const bool value) { return raw(value ? "true" : "false"); }
  bool u32(const std::uint32_t value) { return raw(std::to_string(value)); }
  bool u64(const std::uint64_t value) { return raw(std::to_string(value)); }
  std::string take() && { return std::move(output_); }

 private:
  std::string output_;
};

bool digest_json(Json& json, const FrontendDigest& digest) {
  constexpr char hex[] = "0123456789abcdef";
  std::array<char, 64> value{};
  for (std::size_t index = 0; index < digest.size(); ++index) {
    value[index * 2] = hex[digest[index] >> 4];
    value[index * 2 + 1] = hex[digest[index] & 0x0f];
  }
  return json.string(std::string_view(value.data(), value.size()));
}

std::string_view property_edit_name(const FrontendPropertyEdit value) noexcept {
  switch (value) {
    case FrontendPropertyEdit::edit_anywhere:
      return "edit_anywhere";
    case FrontendPropertyEdit::edit_instance_only:
      return "edit_instance_only";
    case FrontendPropertyEdit::edit_defaults_only:
      return "edit_defaults_only";
    case FrontendPropertyEdit::not_editable:
      return "not_editable";
  }
  return {};
}

std::string_view property_blueprint_name(const FrontendPropertyBlueprint value) noexcept {
  switch (value) {
    case FrontendPropertyBlueprint::blueprint_read_write:
      return "blueprint_read_write";
    case FrontendPropertyBlueprint::blueprint_read_only:
      return "blueprint_read_only";
    case FrontendPropertyBlueprint::blueprint_hidden:
      return "blueprint_hidden";
  }
  return {};
}

std::string_view static_class_name(const FrontendStaticClassMode value) noexcept {
  switch (value) {
    case FrontendStaticClassMode::allowed:
      return "allowed";
    case FrontendStaticClassMode::deprecated:
      return "deprecated";
    case FrontendStaticClassMode::disallowed:
      return "disallowed";
  }
  return {};
}

std::string_view native_kind_name(const FrontendNativeSuperKind value) noexcept {
  switch (value) {
    case FrontendNativeSuperKind::actor:
      return "actor";
    case FrontendNativeSuperKind::actor_component:
      return "actor_component";
    case FrontendNativeSuperKind::engine_subsystem:
      return "engine_subsystem";
    case FrontendNativeSuperKind::editor_subsystem:
      return "editor_subsystem";
    case FrontendNativeSuperKind::game_instance_subsystem:
      return "game_instance_subsystem";
    case FrontendNativeSuperKind::world_subsystem:
      return "world_subsystem";
    case FrontendNativeSuperKind::local_player_subsystem:
      return "local_player_subsystem";
    case FrontendNativeSuperKind::other_uobject:
      // Rust's serde(rename_all = "snake_case") spells OtherUObject with a word
      // boundary before UObject. Keep the wire token identical to the profile ABI.
      return "other_u_object";
  }
  return {};
}

bool graph_profile_json(
    Json& json,
    const bool bound,
    const std::vector<FrontendGraphCapture>& captures) {
  if (!json.raw("{\"bound\":" ) || !json.boolean(bound) || !json.raw(",\"captures\":[")) {
    return false;
  }
  for (std::size_t index = 0; index < captures.size(); ++index) {
    const auto& capture = captures[index];
    if (capture.ordinal != index || (index != 0 && !json.raw(",")) ||
        !json.raw("{\"ordinal\":") || !json.u32(capture.ordinal) ||
        !json.raw(",\"input_graph_sha256\":") ||
        !digest_json(json, capture.input_graph_sha256) ||
        !json.raw(",\"output_graph_sha256\":") ||
        !digest_json(json, capture.output_graph_sha256) || !json.raw(",\"modules\":[")) {
      return false;
    }
    for (std::size_t module_index = 0; module_index < capture.modules.size(); ++module_index) {
      const auto& module = capture.modules[module_index];
      if ((module_index != 0 && !json.raw(",")) || !json.raw("{\"ordinal\":") ||
          !json.u32(static_cast<std::uint32_t>(module_index)) ||
          !json.raw(",\"module_name\":") || !json.string(module.module_name) ||
          !json.raw(",\"generated_declarations\":") ||
          !json.string(module.generated_declarations) || !json.raw("}")) {
        return false;
      }
    }
    if (!json.raw("]}")) return false;
  }
  return json.raw("]}");
}

bool preprocessor_json(
    const FrontendPreprocessorConfig& config,
    const FrontendDigest& digest,
    std::string& output) {
  Json json;
  if (!json.raw("{\"schema\":\"gore.as.preprocessor-config\",\"schema_version\":1,"
                "\"automatic_imports\":") ||
      !json.boolean(config.automatic_imports) ||
      !json.raw(",\"warn_on_manual_import_statements\":") ||
      !json.boolean(config.warn_on_manual_import_statements) ||
      !json.raw(",\"use_editor_scripts\":") || !json.boolean(config.use_editor_scripts) ||
      !json.raw(",\"effective_flags\":[")) {
    return false;
  }
  for (std::size_t index = 0; index < config.effective_flags.size(); ++index) {
    const auto& flag = config.effective_flags[index];
    if ((index != 0 && !json.raw(",")) || !json.raw("{\"ordinal\":") ||
        !json.u32(static_cast<std::uint32_t>(index)) || !json.raw(",\"name\":") ||
        !json.string(flag.name) || !json.raw(",\"value\":") || !json.boolean(flag.value) ||
        !json.raw("}")) {
      return false;
    }
  }
  const auto edit = property_edit_name(config.default_property_edit_specifier);
  const auto struct_edit =
      property_edit_name(config.default_property_edit_specifier_for_structs);
  const auto blueprint =
      property_blueprint_name(config.default_property_blueprint_specifier);
  const auto static_mode = static_class_name(config.static_class_mode);
  if (edit.empty() || struct_edit.empty() || blueprint.empty() || static_mode.empty() ||
      !json.raw("],\"default_function_blueprint_callable\":") ||
      !json.boolean(config.default_function_blueprint_callable) ||
      !json.raw(",\"default_property_edit_specifier\":") || !json.string(edit) ||
      !json.raw(",\"default_property_edit_specifier_for_structs\":") ||
      !json.string(struct_edit) ||
      !json.raw(",\"default_property_blueprint_specifier\":") ||
      !json.string(blueprint) || !json.raw(",\"static_class_mode\":") ||
      !json.string(static_mode) || !json.raw(",\"script_float_is_float64\":") ||
      !json.boolean(config.script_float_is_float64) ||
      !json.raw(",\"angelscript_haze\":") || !json.boolean(config.angelscript_haze) ||
      !json.raw(",\"enforce_server_rpc_validation\":") ||
      !json.boolean(config.enforce_server_rpc_validation) ||
      !json.raw(",\"blueprint_event_argument_specializations\":[")) {
    return false;
  }
  for (std::size_t index = 0;
       index < config.blueprint_event_argument_specializations.size();
       ++index) {
    if ((index != 0 && !json.raw(",")) ||
        !json.string(config.blueprint_event_argument_specializations[index])) {
      return false;
    }
  }
  if (!json.raw("],\"native_super_types\":[")) return false;
  for (std::size_t index = 0; index < config.native_super_types.size(); ++index) {
    const auto& native = config.native_super_types[index];
    const auto kind = native_kind_name(native.kind);
    if (kind.empty() || (index != 0 && !json.raw(",")) ||
        !json.raw("{\"ordinal\":") || !json.u32(static_cast<std::uint32_t>(index)) ||
        !json.raw(",\"angelscript_type_name\":") ||
        !json.string(native.angelscript_type_name) ||
        !json.raw(",\"unreal_class_path\":") || !json.string(native.unreal_class_path) ||
        !json.raw(",\"property_offset\":") || !json.u64(native.property_offset) ||
        !json.raw(",\"kind\":") || !json.string(kind) ||
        !json.raw(",\"game_state_subsystem\":") ||
        !json.boolean(native.game_state_subsystem) ||
        !json.raw(",\"cannot_derive_angelscript\":") ||
        !json.boolean(native.cannot_derive_angelscript) || !json.raw("}")) {
      return false;
    }
  }
  if (!json.raw("],\"fname_comparison_keys\":[")) return false;
  for (std::size_t index = 0; index < config.fname_comparison_keys.size(); ++index) {
    const auto& key = config.fname_comparison_keys[index];
    if ((index != 0 && !json.raw(",")) || !json.raw("{\"ordinal\":") ||
        !json.u32(static_cast<std::uint32_t>(index)) || !json.raw(",\"spelling\":") ||
        !json.string(key.spelling) || !json.raw(",\"comparison_key\":") ||
        !json.string(key.comparison_key) || !json.raw("}")) {
      return false;
    }
  }
  if (!json.raw("],\"external_hooks\":{\"class_analyze\":{\"bound\":" ) ||
      !json.boolean(config.class_analyze_bound) || !json.raw(",\"captures\":[")) {
    return false;
  }
  for (std::size_t index = 0; index < config.class_analyze_captures.size(); ++index) {
    const auto& capture = config.class_analyze_captures[index];
    if (capture.ordinal != index || (index != 0 && !json.raw(",")) ||
        !json.raw("{\"ordinal\":") || !json.u32(capture.ordinal) ||
        !json.raw(",\"module_name\":") || !json.string(capture.module_name) ||
        !json.raw(",\"namespace\":") || !json.string(capture.name_space) ||
        !json.raw(",\"class_name\":") || !json.string(capture.class_name) ||
        !json.raw(",\"source_sha256\":") || !digest_json(json, capture.source_sha256) ||
        !json.raw(",\"input_generated_statics_sha256\":") ||
        !digest_json(json, capture.input_generated_statics_sha256) ||
        !json.raw(",\"generated_statics\":") || !json.string(capture.generated_statics) ||
        !json.raw(",\"output_generated_statics_sha256\":") ||
        !digest_json(json, capture.output_generated_statics_sha256) ||
        !json.raw(",\"has_statics\":") || !json.boolean(capture.has_statics) ||
        !json.raw(",\"compose_onto_class\":") ||
        !json.string(capture.compose_onto_class) || !json.raw("}")) {
      return false;
    }
  }
  if (!json.raw("]},\"process_chunks\":") ||
      !graph_profile_json(json, config.process_chunks_bound, config.process_chunks_captures) ||
      !json.raw(",\"post_process_code\":") ||
      !graph_profile_json(
          json, config.post_process_code_bound, config.post_process_code_captures) ||
      !json.raw("},\"canonical_sha256\":") || !digest_json(json, digest) ||
      !json.raw("}")) {
    return false;
  }
  output = std::move(json).take();
  return true;
}

bool canonical_digest(
    const std::string_view domain,
    const std::string_view zeroed_json,
    FrontendDigest& digest) noexcept {
  Sha256 hash;
  return hash.append(domain) && append_u64(hash, zeroed_json.size()) &&
         hash.append(zeroed_json) && hash.finish(digest);
}

bool read_bool(const std::byte* const bytes, const std::size_t offset, bool& value) noexcept {
  const auto raw = std::to_integer<std::uint8_t>(bytes[offset]);
  if (raw > 1) return false;
  value = raw != 0;
  return true;
}

}  // namespace

FrontendObserverError project_frontend_settings_v1(
    const std::byte* const settings,
    const std::size_t settings_bytes,
    const std::byte* const preprocessor,
    const std::size_t preprocessor_bytes,
    const bool automatic_imports,
    const bool use_editor_scripts,
    FrontendPreprocessorConfig& preprocessor_config,
    FrontendClassGeneratorConfig& class_generator_config,
    FrontendCompilerOptions& compiler_options) noexcept {
  using namespace frontend_target_layout;
  if (settings == nullptr || preprocessor == nullptr || settings_bytes <= settings_warn_complex_increment ||
      preprocessor_bytes <= preprocessor_default_property_blueprint) {
    return FrontendObserverError::invalid_argument;
  }
  FrontendPreprocessorConfig pre = preprocessor_config;
  FrontendClassGeneratorConfig generator{};
  FrontendCompilerOptions options{};
  pre.automatic_imports = automatic_imports;
  pre.use_editor_scripts = use_editor_scripts;
  if (!read_bool(settings, settings_warn_manual_imports, pre.warn_on_manual_import_statements) ||
      !read_bool(
          preprocessor,
          preprocessor_default_function_blueprint,
          pre.default_function_blueprint_callable) ||
      !read_bool(settings, settings_script_float64, pre.script_float_is_float64) ||
      !read_bool(
          settings,
          settings_mark_non_uproperty_transient,
          generator.mark_non_uproperty_properties_as_transient) ||
      !read_bool(settings, settings_error_editor_only, options.error_on_incorrect_editor_only_code) ||
      !read_bool(
          settings,
          settings_warn_divergent_comparison,
          options.warn_on_divergent_comparison_operator_overloads) ||
      !read_bool(
          settings,
          settings_warn_signed_unsigned,
          options.warn_on_implicit_signed_unsigned_conversion) ||
      !read_bool(
          settings,
          settings_warn_complex_increment,
          options.warn_on_increment_decrement_in_complex_expression) ||
      !read_bool(
          settings,
          settings_warn_unused_const_return,
          options.warn_on_unused_return_value_for_const_methods)) {
    return FrontendObserverError::invalid_target_value;
  }
  const auto edit = std::to_integer<std::uint8_t>(
      preprocessor[preprocessor_default_property_edit]);
  const auto struct_edit = std::to_integer<std::uint8_t>(
      preprocessor[preprocessor_default_struct_property_edit]);
  const auto blueprint = std::to_integer<std::uint8_t>(
      preprocessor[preprocessor_default_property_blueprint]);
  const auto static_mode = std::to_integer<std::uint8_t>(settings[settings_static_class_mode]);
  if (edit > static_cast<std::uint8_t>(FrontendPropertyEdit::not_editable) ||
      struct_edit > static_cast<std::uint8_t>(FrontendPropertyEdit::not_editable) ||
      blueprint > static_cast<std::uint8_t>(FrontendPropertyBlueprint::blueprint_hidden) ||
      static_mode > static_cast<std::uint8_t>(FrontendStaticClassMode::disallowed)) {
    return FrontendObserverError::invalid_target_value;
  }
  pre.default_property_edit_specifier = static_cast<FrontendPropertyEdit>(edit);
  pre.default_property_edit_specifier_for_structs =
      static_cast<FrontendPropertyEdit>(struct_edit);
  pre.default_property_blueprint_specifier =
      static_cast<FrontendPropertyBlueprint>(blueprint);
  pre.static_class_mode = static_cast<FrontendStaticClassMode>(static_mode);
  // These two Shipping compile-time values have independent target binary witnesses.
  pre.angelscript_haze = false;
  pre.enforce_server_rpc_validation = false;
  preprocessor_config = std::move(pre);
  class_generator_config = generator;
  compiler_options = options;
  return FrontendObserverError::ok;
}

FrontendObserverError derive_native_super_v1(
    const FrontendNativeClassWitness& witness,
    FrontendNativeSuper& projection) noexcept {
  if (!valid_utf8(witness.angelscript_type_name, false, 4096, true) ||
      !valid_utf8(witness.unreal_class_path, false, 4096, true) ||
      witness.property_offset > static_cast<std::uint64_t>(std::numeric_limits<std::int32_t>::max()) ||
      witness.ancestry_paths.empty() || witness.ancestry_paths.size() > 1024 ||
      witness.ancestry_paths.front() != witness.unreal_class_path) {
    return FrontendObserverError::invalid_argument;
  }
  FrontendNativeSuperKind kind = FrontendNativeSuperKind::other_uobject;
  const auto has = [&](const std::string_view path) {
    return std::find(witness.ancestry_paths.begin(), witness.ancestry_paths.end(), path) !=
           witness.ancestry_paths.end();
  };
  const bool game_state_subsystem =
      has("/Script/GameStateSubsystem.GameStateSubsystem");
  for (const auto& path : witness.ancestry_paths) {
    if (!valid_utf8(path, false, 4096, true)) {
      return FrontendObserverError::invalid_utf8;
    }
  }
  if (has("/Script/Engine.Actor")) {
    kind = FrontendNativeSuperKind::actor;
  } else if (has("/Script/Engine.ActorComponent")) {
    kind = FrontendNativeSuperKind::actor_component;
  } else if (has("/Script/Engine.EngineSubsystem")) {
    kind = FrontendNativeSuperKind::engine_subsystem;
  } else if (has("/Script/UnrealEd.EditorSubsystem")) {
    kind = FrontendNativeSuperKind::editor_subsystem;
  } else if (has("/Script/Engine.GameInstanceSubsystem")) {
    kind = FrontendNativeSuperKind::game_instance_subsystem;
  } else if (has("/Script/Engine.WorldSubsystem")) {
    kind = FrontendNativeSuperKind::world_subsystem;
  } else if (has("/Script/Engine.LocalPlayerSubsystem")) {
    kind = FrontendNativeSuperKind::local_player_subsystem;
  } else if (!has("/Script/CoreUObject.Object")) {
    return FrontendObserverError::unresolved_semantics;
  }
  projection = {
      witness.angelscript_type_name,
      witness.unreal_class_path,
      witness.property_offset,
      kind,
      game_state_subsystem,
      false};  // CannotDeriveAngelscript is inside the target's WITH_EDITOR guard.
  return FrontendObserverError::ok;
}

FrontendObserverError make_fname_comparison_key_v1(
    const std::string_view spelling,
    const std::uint32_t target_comparison_index,
    FrontendFNameComparison& projection) noexcept {
  if (!valid_utf8(spelling, false, 4096, true)) {
    return FrontendObserverError::invalid_utf8;
  }
  // ASCII identities are reproduced exactly by the standalone ASCII fold and must not be
  // smuggled into this target-only table. Only spellings that need Unreal's comparison index
  // belong here.
  if (std::none_of(spelling.begin(), spelling.end(), [](const char value) {
        return static_cast<unsigned char>(value) >= 0x80u;
      })) {
    return FrontendObserverError::invalid_argument;
  }
  constexpr char hex[] = "0123456789abcdef";
  std::array<char, 8> encoded{};
  for (std::size_t index = 0; index < encoded.size(); ++index) {
    const auto shift = static_cast<unsigned>((encoded.size() - index - 1) * 4);
    encoded[index] = hex[(target_comparison_index >> shift) & 0x0f];
  }
  projection.spelling.assign(spelling);
  projection.comparison_key = "ue5-fname-comparison-index-v1:";
  projection.comparison_key.append(encoded.data(), encoded.size());
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::set_hook_bindings(
    const bool class_analyze,
    const bool process_chunks,
    const bool post_process_code) noexcept {
  if (bindings_set_ || pending_ != PendingKind::none) return FrontendObserverError::invalid_order;
  bindings_set_ = true;
  class_bound_ = class_analyze;
  process_bound_ = process_chunks;
  post_bound_ = post_process_code;
  // A bound ClassAnalyze delegate may legitimately see no source classes. Binding is the
  // observation proof in that case; individual invocations are still bracketed below.
  class_observed_ = true;
  // Even an unbound delegate is broadcast at its pinned callsite. The dispatcher must bracket
  // that exact call so transient binding/header drift and missing/reordered callsites cannot be
  // hidden by identical graph bytes.
  process_observed_ = false;
  post_observed_ = false;
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::begin_class_analyze(
    const FrontendClassFrame& frame) noexcept {
  if (!bindings_set_ || finished_ || !class_bound_ || pending_ != PendingKind::none ||
      !valid_utf8(frame.module_name, false, 4096) ||
      !valid_utf8(frame.name_space, true, 4096) ||
      !valid_utf8(frame.class_name, false, 4096) || !valid_utf8(frame.source) ||
      !valid_utf8(frame.generated_statics) ||
      !valid_utf8(frame.compose_onto_class, true, 4096)) {
    return FrontendObserverError::invalid_argument;
  }
  try {
    pending_class_ = frame;
  } catch (...) {
    return FrontendObserverError::limit_exceeded;
  }
  pending_ = PendingKind::class_analyze;
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::complete_class_analyze(
    const FrontendClassFrame& frame) noexcept {
  if (pending_ != PendingKind::class_analyze ||
      pending_class_.module_name != frame.module_name ||
      pending_class_.name_space != frame.name_space ||
      pending_class_.class_name != frame.class_name || pending_class_.source != frame.source ||
      !valid_utf8(frame.generated_statics) ||
      !valid_utf8(frame.compose_onto_class, true, 4096)) {
    return FrontendObserverError::invalid_order;
  }
  for (const auto& capture : class_captures_) {
    if (capture.module_name == frame.module_name && capture.name_space == frame.name_space &&
        capture.class_name == frame.class_name) {
      return FrontendObserverError::duplicate_identity;
    }
  }
  FrontendClassCapture capture{};
  capture.ordinal = static_cast<std::uint32_t>(class_captures_.size());
  capture.module_name = frame.module_name;
  capture.name_space = frame.name_space;
  capture.class_name = frame.class_name;
  capture.generated_statics = frame.generated_statics;
  capture.has_statics = frame.has_statics;
  capture.compose_onto_class = frame.compose_onto_class;
  if (!sha256_bytes(frame.source, capture.source_sha256) ||
      !sha256_bytes(
          pending_class_.generated_statics, capture.input_generated_statics_sha256) ||
      !sha256_bytes(frame.generated_statics, capture.output_generated_statics_sha256)) {
    return FrontendObserverError::hash_failure;
  }
  try {
    class_captures_.push_back(std::move(capture));
  } catch (...) {
    return FrontendObserverError::limit_exceeded;
  }
  pending_ = PendingKind::none;
  pending_class_ = {};
  class_observed_ = true;
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::begin_graph(
    const PendingKind kind,
    const std::vector<FrontendGraphModule>& modules) noexcept {
  if (!bindings_set_ || finished_ || pending_ != PendingKind::none ||
      !valid_module_graph(modules)) {
    return FrontendObserverError::invalid_argument;
  }
  if (!graph_input_digest(modules, pending_graph_digest_)) {
    return FrontendObserverError::hash_failure;
  }
  try {
    pending_modules_ = modules;
  } catch (...) {
    return FrontendObserverError::limit_exceeded;
  }
  pending_ = kind;
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::complete_graph(
    const PendingKind kind,
    const std::vector<FrontendGraphModule>& modules,
    std::vector<FrontendGraphCapture>& captures) noexcept {
  if (pending_ != kind || !valid_module_graph(modules) ||
      !same_graph_input(pending_modules_, modules)) {
    return FrontendObserverError::invalid_order;
  }
  const bool bound = kind == PendingKind::process_chunks ? process_bound_ : post_bound_;
  if (!bound) {
    // An unbound delegate does not imply that generated declarations are globally empty:
    // ClassAnalyze may already have populated them. It proves exact identity across this
    // bracket, including the output-only declaration field.
    for (std::size_t index = 0; index < modules.size(); ++index) {
      if (modules[index].generated_declarations !=
          pending_modules_[index].generated_declarations) {
        return FrontendObserverError::invalid_order;
      }
    }
  } else {
    if (std::any_of(captures.begin(), captures.end(), [&](const auto& capture) {
          return capture.input_graph_sha256 == pending_graph_digest_;
        })) {
      return FrontendObserverError::duplicate_identity;
    }
    FrontendGraphCapture capture{};
    capture.ordinal = static_cast<std::uint32_t>(captures.size());
    capture.input_graph_sha256 = pending_graph_digest_;
    capture.modules = modules;
    if (!graph_output_digest(
            capture.input_graph_sha256, capture.modules, capture.output_graph_sha256)) {
      return FrontendObserverError::hash_failure;
    }
    try {
      captures.push_back(std::move(capture));
    } catch (...) {
      return FrontendObserverError::limit_exceeded;
    }
  }
  pending_ = PendingKind::none;
  pending_modules_.clear();
  pending_graph_digest_ = {};
  if (kind == PendingKind::process_chunks) {
    process_observed_ = true;
  } else {
    post_observed_ = true;
  }
  return FrontendObserverError::ok;
}

FrontendObserverError FrontendSemanticObserver::begin_process_chunks(
    const std::vector<FrontendGraphModule>& modules) noexcept {
  return begin_graph(PendingKind::process_chunks, modules);
}

FrontendObserverError FrontendSemanticObserver::complete_process_chunks(
    const std::vector<FrontendGraphModule>& modules) noexcept {
  return complete_graph(PendingKind::process_chunks, modules, process_captures_);
}

FrontendObserverError FrontendSemanticObserver::begin_post_process_code(
    const std::vector<FrontendGraphModule>& modules) noexcept {
  return begin_graph(PendingKind::post_process, modules);
}

FrontendObserverError FrontendSemanticObserver::complete_post_process_code(
    const std::vector<FrontendGraphModule>& modules) noexcept {
  return complete_graph(PendingKind::post_process, modules, post_captures_);
}

FrontendObserverError FrontendSemanticObserver::finish(
    FrontendPreprocessorConfig& config) noexcept {
  if (!bindings_set_ || finished_ || pending_ != PendingKind::none || !class_observed_ ||
      !process_observed_ || !post_observed_) {
    return FrontendObserverError::invalid_order;
  }
  config.class_analyze_bound = class_bound_;
  config.process_chunks_bound = process_bound_;
  config.post_process_code_bound = post_bound_;
  config.class_analyze_captures = std::move(class_captures_);
  config.process_chunks_captures = std::move(process_captures_);
  config.post_process_code_captures = std::move(post_captures_);
  finished_ = true;
  return FrontendObserverError::ok;
}

FrontendObserverError serialize_preprocessor_config_json_v1(
    FrontendPreprocessorConfig& config,
    std::string& json) noexcept {
  if (config.effective_flags.size() > kMaximumConfigurationItems ||
      config.blueprint_event_argument_specializations.size() >
          kMaximumConfigurationItems ||
      config.native_super_types.size() > kMaximumItems ||
      config.fname_comparison_keys.size() > kMaximumItems ||
      (!config.class_analyze_bound && !config.class_analyze_captures.empty()) ||
      (!config.process_chunks_bound && !config.process_chunks_captures.empty()) ||
      (!config.post_process_code_bound && !config.post_process_code_captures.empty()) ||
      !valid_class_captures(config.class_analyze_captures) ||
      !valid_graph_captures(config.process_chunks_captures) ||
      !valid_graph_captures(config.post_process_code_captures)) {
    return FrontendObserverError::invalid_argument;
  }
  const std::array<std::string_view, 6> required{
      "COOK_COMMANDLET", "EDITOR", "EDITORONLY_DATA", "RELEASE", "TEST", "WITH_SERVER_CODE"};
  std::string_view previous;
  for (const auto& flag : config.effective_flags) {
    if (!valid_utf8(flag.name, false, 4096, true) ||
        (!previous.empty() && previous >= flag.name)) {
      return FrontendObserverError::invalid_order;
    }
    previous = flag.name;
  }
  for (const auto name : required) {
    if (std::none_of(config.effective_flags.begin(), config.effective_flags.end(),
                     [&](const auto& flag) { return flag.name == name; })) {
      return FrontendObserverError::unresolved_semantics;
    }
  }
  const auto sorted_strings = [](const std::vector<std::string>& values) {
    for (std::size_t index = 0; index < values.size(); ++index) {
      if (!valid_utf8(values[index], false, 4096, true) ||
          (index != 0 && values[index - 1] >= values[index])) {
        return false;
      }
    }
    return true;
  };
  if (!sorted_strings(config.blueprint_event_argument_specializations)) {
    return FrontendObserverError::invalid_order;
  }
  std::set<std::string_view> native_paths;
  for (std::size_t index = 0; index < config.native_super_types.size(); ++index) {
    const auto& value = config.native_super_types[index];
    if (!valid_utf8(value.angelscript_type_name, false, 4096, true) ||
        !valid_utf8(value.unreal_class_path, false, 4096, true) ||
        value.property_offset > static_cast<std::uint64_t>(std::numeric_limits<std::int32_t>::max()) ||
        !native_paths.insert(value.unreal_class_path).second ||
        (index != 0 && config.native_super_types[index - 1].angelscript_type_name >=
                           value.angelscript_type_name)) {
      return FrontendObserverError::invalid_order;
    }
  }
  for (std::size_t index = 0; index < config.fname_comparison_keys.size(); ++index) {
    const auto& value = config.fname_comparison_keys[index];
    if (!valid_utf8(value.spelling, false, 4096, true) ||
        !valid_utf8(value.comparison_key, false, 4096, true) ||
        std::none_of(value.spelling.begin(), value.spelling.end(), [](const char byte) {
          return static_cast<unsigned char>(byte) >= 0x80u;
        }) ||
        (index != 0 && config.fname_comparison_keys[index - 1].spelling >= value.spelling)) {
      return FrontendObserverError::invalid_order;
    }
  }
  FrontendDigest zero{};
  std::string zeroed;
  if (!preprocessor_json(config, zero, zeroed) ||
      !canonical_digest(
          std::string_view(kPreprocessorDomain, sizeof(kPreprocessorDomain) - 1),
          zeroed,
          config.canonical_sha256) ||
      !preprocessor_json(config, config.canonical_sha256, json)) {
    return FrontendObserverError::hash_failure;
  }
  return FrontendObserverError::ok;
}

FrontendObserverError serialize_class_generator_config_json_v1(
    FrontendClassGeneratorConfig& config,
    std::string& json) noexcept {
  const auto make = [&](const FrontendDigest& digest, std::string& output) {
    Json writer;
    if (!writer.raw(
            "{\"schema\":\"gore.as.class-generator-config\",\"schema_version\":1,"
            "\"mark_non_uproperty_properties_as_transient\":") ||
        !writer.boolean(config.mark_non_uproperty_properties_as_transient) ||
        !writer.raw(",\"canonical_sha256\":") || !digest_json(writer, digest) ||
        !writer.raw("}")) {
      return false;
    }
    output = std::move(writer).take();
    return true;
  };
  FrontendDigest zero{};
  std::string zeroed;
  if (!make(zero, zeroed) ||
      !canonical_digest(
          std::string_view(kClassGeneratorDomain, sizeof(kClassGeneratorDomain) - 1),
          zeroed,
          config.canonical_sha256) ||
      !make(config.canonical_sha256, json)) {
    return FrontendObserverError::hash_failure;
  }
  return FrontendObserverError::ok;
}

FrontendObserverError serialize_compiler_options_json_v1(
    FrontendCompilerOptions& options,
    std::string& json) noexcept {
  const auto make = [&](const FrontendDigest& digest, std::string& output) {
    Json writer;
    if (!writer.raw(
            "{\"schema\":\"gore.as.compiler-options\",\"schema_version\":1,"
            "\"error_on_incorrect_editor_only_code\":") ||
        !writer.boolean(options.error_on_incorrect_editor_only_code) ||
        !writer.raw(",\"warn_on_divergent_comparison_operator_overloads\":") ||
        !writer.boolean(options.warn_on_divergent_comparison_operator_overloads) ||
        !writer.raw(",\"warn_on_implicit_signed_unsigned_conversion\":") ||
        !writer.boolean(options.warn_on_implicit_signed_unsigned_conversion) ||
        !writer.raw(",\"warn_on_increment_decrement_in_complex_expression\":") ||
        !writer.boolean(options.warn_on_increment_decrement_in_complex_expression) ||
        !writer.raw(",\"warn_on_unused_return_value_for_const_methods\":") ||
        !writer.boolean(options.warn_on_unused_return_value_for_const_methods) ||
        !writer.raw(",\"canonical_sha256\":") || !digest_json(writer, digest) ||
        !writer.raw("}")) {
      return false;
    }
    output = std::move(writer).take();
    return true;
  };
  FrontendDigest zero{};
  std::string zeroed;
  if (!make(zero, zeroed) ||
      !canonical_digest(
          std::string_view(kCompilerOptionsDomain, sizeof(kCompilerOptionsDomain) - 1),
          zeroed,
          options.canonical_sha256) ||
      !make(options.canonical_sha256, json)) {
    return FrontendObserverError::hash_failure;
  }
  return FrontendObserverError::ok;
}

FrontendObserverError frontend_config_set_digest_v1(
    const FrontendPreprocessorConfig& preprocessor,
    const FrontendClassGeneratorConfig& class_generator,
    const FrontendCompilerOptions& compiler_options,
    FrontendDigest& digest) noexcept {
  const FrontendDigest zero{};
  if (preprocessor.canonical_sha256 == zero ||
      class_generator.canonical_sha256 == zero ||
      compiler_options.canonical_sha256 == zero) {
    return FrontendObserverError::invalid_argument;
  }
  Sha256 hash;
  if (!hash.append(kFrontendConfigSetDomain, sizeof(kFrontendConfigSetDomain) - 1) ||
      !hash.append(
          preprocessor.canonical_sha256.data(), preprocessor.canonical_sha256.size()) ||
      !hash.append(
          class_generator.canonical_sha256.data(),
          class_generator.canonical_sha256.size()) ||
      !hash.append(
          compiler_options.canonical_sha256.data(),
          compiler_options.canonical_sha256.size()) ||
      !hash.finish(digest)) {
    return FrontendObserverError::hash_failure;
  }
  return FrontendObserverError::ok;
}

FrontendObserverError project_initial_compile_enter_v1(
    const FrontendDigest& config_sha256,
    FrontendBoundaryProjection& boundary) noexcept {
  if (config_sha256 == FrontendDigest{}) return FrontendObserverError::invalid_argument;
  boundary = {};
  boundary.kind = FrontendBoundaryKind::initial_compile_enter;
  boundary.observation_rva = frontend_target_layout::initial_compile_enter_rva;
  boundary.config_sha256 = config_sha256;
  return FrontendObserverError::ok;
}

FrontendObserverError project_graph_boundary(
    const FrontendBoundaryKind kind,
    const std::uint32_t observation_rva,
    const FrontendDigest& config_sha256,
    const std::vector<FrontendGraphModule>& modules,
    FrontendBoundaryProjection& boundary) noexcept {
  if (config_sha256 == FrontendDigest{} || modules.empty() ||
      modules.size() > std::numeric_limits<std::uint32_t>::max() ||
      !valid_module_graph(modules)) {
    return FrontendObserverError::invalid_argument;
  }
  FrontendBoundaryProjection projected{};
  projected.kind = kind;
  projected.observation_rva = observation_rva;
  projected.module_count = static_cast<std::uint32_t>(modules.size());
  projected.config_sha256 = config_sha256;
  if (!graph_input_digest(modules, projected.input_sha256) ||
      !graph_output_digest(
          projected.input_sha256, modules, projected.output_sha256)) {
    return FrontendObserverError::hash_failure;
  }
  boundary = projected;
  return FrontendObserverError::ok;
}

FrontendObserverError project_precompiled_descriptors_v1(
    const FrontendDigest& config_sha256,
    const std::vector<FrontendGraphModule>& modules,
    FrontendBoundaryProjection& boundary) noexcept {
  return project_graph_boundary(
      FrontendBoundaryKind::precompiled_descriptors_requested,
      frontend_target_layout::descriptors_requested_rva,
      config_sha256,
      modules,
      boundary);
}

FrontendObserverError project_preprocessor_constructed_v1(
    const FrontendDigest& config_sha256,
    FrontendBoundaryProjection& boundary) noexcept {
  if (config_sha256 == FrontendDigest{}) return FrontendObserverError::invalid_argument;
  boundary = {};
  boundary.kind = FrontendBoundaryKind::preprocessor_constructed;
  boundary.observation_rva = frontend_target_layout::preprocessor_constructed_rva;
  boundary.config_sha256 = config_sha256;
  return FrontendObserverError::ok;
}

FrontendObserverError project_initial_compile_return_v1(
    const FrontendDigest& config_sha256,
    const std::vector<FrontendGraphModule>& modules,
    FrontendBoundaryProjection& boundary) noexcept {
  return project_graph_boundary(
      FrontendBoundaryKind::initial_compile_return,
      frontend_target_layout::initial_compile_return_rva,
      config_sha256,
      modules,
      boundary);
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_frontend_observer_selftest_v1() noexcept {
  if (native_kind_name(FrontendNativeSuperKind::other_uobject) != "other_u_object") {
    return false;
  }
  std::uint32_t callback_kind_mask = 0;
  for (const auto& site : frontend_target_layout::callback_callsites) {
    std::int32_t encoded_displacement = 0;
    std::memcpy(
        &encoded_displacement,
        site.expected_call.data() + 1,
        sizeof(encoded_displacement));
    const auto kind = static_cast<std::uint32_t>(site.kind);
    if (kind == 0 || kind > 3 || (callback_kind_mask & (1u << (kind - 1))) != 0 ||
        site.expected_call.front() != std::byte{0xe8} ||
        site.return_rva != site.call_rva + site.expected_call.size() ||
        site.relative_displacement != encoded_displacement ||
        static_cast<std::int64_t>(site.return_rva) + encoded_displacement !=
            site.direct_callee_rva ||
        site.call_rva >= kPeSizeOfImage || site.return_rva >= kPeSizeOfImage ||
        site.direct_callee_rva >= kPeSizeOfImage ||
        (site.kind == FrontendCallbackKind::class_analyze) !=
            (site.delegate_storage_rva == 0)) {
      return false;
    }
    callback_kind_mask |= 1u << (kind - 1);
  }
  if (callback_kind_mask != 0x7u) return false;

  std::array<std::byte, 0x80> settings{};
  std::array<std::byte, 0x108> preprocessor{};
  const auto set = [](auto& bytes, const std::size_t offset, const std::uint8_t value) {
    bytes[offset] = static_cast<std::byte>(value);
  };
  set(settings, frontend_target_layout::settings_warn_manual_imports, 1);
  set(settings, frontend_target_layout::settings_script_float64, 1);
  set(settings, frontend_target_layout::settings_error_editor_only, 1);
  set(settings, frontend_target_layout::settings_warn_divergent_comparison, 1);
  set(settings, frontend_target_layout::settings_warn_signed_unsigned, 1);
  set(settings, frontend_target_layout::settings_warn_complex_increment, 1);
  set(settings, frontend_target_layout::settings_warn_unused_const_return, 1);
  set(settings, frontend_target_layout::settings_static_class_mode, 1);
  set(preprocessor, frontend_target_layout::preprocessor_default_function_blueprint, 1);
  set(preprocessor, frontend_target_layout::preprocessor_default_property_edit, 0);
  set(preprocessor, frontend_target_layout::preprocessor_default_struct_property_edit, 2);
  set(preprocessor, frontend_target_layout::preprocessor_default_property_blueprint, 1);

  FrontendPreprocessorConfig config{};
  FrontendClassGeneratorConfig generator{};
  FrontendCompilerOptions options{};
  if (project_frontend_settings_v1(
          settings.data(),
          settings.size(),
          preprocessor.data(),
          preprocessor.size(),
          true,
          false,
          config,
          generator,
          options) != FrontendObserverError::ok ||
      !config.warn_on_manual_import_statements || !config.script_float_is_float64 ||
      config.static_class_mode != FrontendStaticClassMode::deprecated ||
      config.default_property_edit_specifier_for_structs !=
          FrontendPropertyEdit::edit_defaults_only ||
      config.default_property_blueprint_specifier !=
          FrontendPropertyBlueprint::blueprint_read_only ||
      !options.error_on_incorrect_editor_only_code ||
      !options.warn_on_divergent_comparison_operator_overloads ||
      !options.warn_on_implicit_signed_unsigned_conversion ||
      !options.warn_on_increment_decrement_in_complex_expression ||
      !options.warn_on_unused_return_value_for_const_methods) {
    return false;
  }

  FrontendNativeSuper native{};
  if (derive_native_super_v1(
          {"AModActor",
           "/Script/Mod.ModActor",
           312,
           {"/Script/Mod.ModActor", "/Script/Engine.Actor", "/Script/CoreUObject.Object"}},
          native) != FrontendObserverError::ok ||
      native.kind != FrontendNativeSuperKind::actor || native.game_state_subsystem ||
      native.cannot_derive_angelscript) {
    return false;
  }
  config.native_super_types.push_back(std::move(native));
  FrontendFNameComparison fname{};
  if (make_fname_comparison_key_v1("Gr\xc3\xb6\xc3\x9f\x65", 0x1234abcd, fname) !=
          FrontendObserverError::ok ||
      fname.comparison_key != "ue5-fname-comparison-index-v1:1234abcd" ||
      make_fname_comparison_key_v1("ASCII", 1, fname) !=
          FrontendObserverError::invalid_argument) {
    return false;
  }
  config.fname_comparison_keys.push_back(std::move(fname));
  config.effective_flags = {
      {"COOK_COMMANDLET", false},
      {"EDITOR", false},
      {"EDITORONLY_DATA", false},
      {"RELEASE", true},
      {"TEST", false},
      {"WITH_SERVER_CODE", true},
  };
  config.blueprint_event_argument_specializations = {"FName", "int32"};

  FrontendSemanticObserver observer;
  if (observer.set_hook_bindings(true, true, true) != FrontendObserverError::ok) return false;
  FrontendClassFrame before{
      "Mods.Sample", "Example", "AModActor", "class AModActor : AActor {}", "before", false, {}};
  FrontendClassFrame after = before;
  after.generated_statics = "after";
  after.has_statics = true;
  after.compose_onto_class = "AParent";
  if (observer.begin_class_analyze(before) != FrontendObserverError::ok ||
      observer.complete_class_analyze(after) != FrontendObserverError::ok) {
    return false;
  }
  std::vector<FrontendGraphModule> graph{{
      "Mods.Sample", {{"Mods/Sample.as", "class AModActor : AActor {}"}}, {}}};
  if (observer.begin_process_chunks(graph) != FrontendObserverError::ok) return false;
  graph[0].generated_declarations = "void ProcessHook();";
  if (observer.complete_process_chunks(graph) != FrontendObserverError::ok) return false;
  graph[0].generated_declarations.clear();
  if (observer.begin_post_process_code(graph) != FrontendObserverError::ok) return false;
  graph[0].generated_declarations = "void PostHook();";
  if (observer.complete_post_process_code(graph) != FrontendObserverError::ok ||
      observer.finish(config) != FrontendObserverError::ok ||
      config.class_analyze_captures.size() != 1 ||
      config.process_chunks_captures.size() != 1 ||
      config.post_process_code_captures.size() != 1) {
    return false;
  }
  if (observer.finish(config) != FrontendObserverError::invalid_order) return false;
  std::string pre_json;
  std::string generator_json;
  std::string options_json;
  if (serialize_preprocessor_config_json_v1(config, pre_json) != FrontendObserverError::ok ||
      serialize_class_generator_config_json_v1(generator, generator_json) !=
          FrontendObserverError::ok ||
      serialize_compiler_options_json_v1(options, options_json) !=
          FrontendObserverError::ok ||
      pre_json.find("\"process_chunks\":{\"bound\":true") == std::string::npos ||
      pre_json.find("ue5-fname-comparison-index-v1:1234abcd") == std::string::npos ||
      // These are independent serde_json/SHA-256 known answers for the exact Rust field order.
      pre_json.find("56954fb06fb533f682c8750cd40c733735a2b371db5866ad1d1c2f9efcb66287") ==
          std::string::npos ||
      generator_json.find(
          "1750c8a1401454dda70b3bae901c4e720aed885f96353fe520622eebe2aec383") ==
          std::string::npos ||
      options_json.find(
          "76de3966d085b35e93c9c54f62b89cb38ab66b841950302691a46af149991791") ==
          std::string::npos) {
    return false;
  }

  FrontendDigest config_set{};
  Json config_set_json;
  if (frontend_config_set_digest_v1(config, generator, options, config_set) !=
          FrontendObserverError::ok ||
      !digest_json(config_set_json, config_set) ||
      std::move(config_set_json).take() !=
          "\"d26c349271f324e324455790cf09ca142b342cbb929000358a94d46eb2b7004a\"") {
    return false;
  }
  FrontendBoundaryProjection entry{};
  FrontendBoundaryProjection descriptors{};
  FrontendBoundaryProjection constructed{};
  FrontendBoundaryProjection returned{};
  if (project_initial_compile_enter_v1(config_set, entry) != FrontendObserverError::ok ||
      project_precompiled_descriptors_v1(config_set, graph, descriptors) !=
          FrontendObserverError::ok ||
      project_preprocessor_constructed_v1(config_set, constructed) !=
          FrontendObserverError::ok ||
      project_initial_compile_return_v1(config_set, graph, returned) !=
          FrontendObserverError::ok ||
      entry.observation_rva != frontend_target_layout::initial_compile_enter_rva ||
      descriptors.module_count != 1 ||
      descriptors.input_sha256 != returned.input_sha256 ||
      descriptors.output_sha256 != returned.output_sha256 ||
      constructed.observation_rva !=
          frontend_target_layout::preprocessor_constructed_rva ||
      project_precompiled_descriptors_v1(config_set, {}, descriptors) !=
          FrontendObserverError::invalid_argument) {
    return false;
  }

  auto invalid_config = config;
  invalid_config.process_chunks_captures[0].output_graph_sha256[0] ^= 1;
  std::string ignored_json;
  if (serialize_preprocessor_config_json_v1(invalid_config, ignored_json) !=
      FrontendObserverError::invalid_argument) {
    return false;
  }
  invalid_config = config;
  invalid_config.native_super_types.push_back(invalid_config.native_super_types.front());
  invalid_config.native_super_types.back().angelscript_type_name = "ZDuplicatePath";
  if (serialize_preprocessor_config_json_v1(invalid_config, ignored_json) !=
      FrontendObserverError::invalid_order) {
    return false;
  }

  FrontendSemanticObserver corrupt;
  if (corrupt.set_hook_bindings(false, false, false) != FrontendObserverError::ok) return false;
  graph[0].generated_declarations.clear();
  if (corrupt.begin_process_chunks(graph) != FrontendObserverError::ok) return false;
  graph[0].generated_declarations = "forbidden";
  if (corrupt.complete_process_chunks(graph) != FrontendObserverError::invalid_order) {
    return false;
  }
  settings[frontend_target_layout::settings_warn_manual_imports] = std::byte{2};
  return project_frontend_settings_v1(
             settings.data(),
             settings.size(),
             preprocessor.data(),
             preprocessor.size(),
             true,
             false,
             config,
             generator,
             options) == FrontendObserverError::invalid_target_value;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
