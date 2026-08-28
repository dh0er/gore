#pragma once

#include "gore_as_capture/format.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

using FrontendDigest = std::array<std::uint8_t, 32>;

enum class FrontendObserverError : std::uint32_t {
  ok = 0,
  invalid_argument,
  invalid_target_value,
  invalid_utf8,
  invalid_order,
  duplicate_identity,
  unresolved_semantics,
  limit_exceeded,
  hash_failure,
};

enum class FrontendPropertyEdit : std::uint8_t {
  edit_anywhere = 0,
  edit_instance_only = 1,
  edit_defaults_only = 2,
  not_editable = 3,
};

enum class FrontendPropertyBlueprint : std::uint8_t {
  blueprint_read_write = 0,
  blueprint_read_only = 1,
  blueprint_hidden = 2,
};

enum class FrontendStaticClassMode : std::uint8_t {
  allowed = 0,
  deprecated = 1,
  disallowed = 2,
};

enum class FrontendNativeSuperKind : std::uint8_t {
  actor = 0,
  actor_component,
  engine_subsystem,
  editor_subsystem,
  game_instance_subsystem,
  world_subsystem,
  local_player_subsystem,
  other_uobject,
};

struct FrontendFlag final {
  std::string name;
  bool value{};
};

struct FrontendNativeSuper final {
  std::string angelscript_type_name;
  std::string unreal_class_path;
  std::uint64_t property_offset{};
  FrontendNativeSuperKind kind{FrontendNativeSuperKind::other_uobject};
  // Exact target predicate consumed by G1R's bound ClassAnalyze delegate.  It is
  // intentionally independent of the donor's built-in subsystem categories.
  bool game_state_subsystem{};
  bool cannot_derive_angelscript{};
};

struct FrontendFNameComparison final {
  std::string spelling;
  std::string comparison_key;
};

// Pointer-free witness produced after a target UClass has been traversed through the exact
// UObject Name/Outer and UStruct SuperStruct/PropertiesSize fields.  The observer derives the
// category from the complete path chain; callers do not supply a category enum.
struct FrontendNativeClassWitness final {
  std::string angelscript_type_name;
  std::string unreal_class_path;
  std::uint64_t property_offset{};
  std::vector<std::string> ancestry_paths;
};

struct FrontendGraphSection final {
  std::string relative_path;
  std::string code;
};

struct FrontendGraphModule final {
  std::string module_name;
  std::vector<FrontendGraphSection> sections;
  // The exact replayable append made by the external hook. Empty is meaningful.
  std::string generated_declarations;
};

struct FrontendGraphCapture final {
  std::uint32_t ordinal{};
  FrontendDigest input_graph_sha256{};
  FrontendDigest output_graph_sha256{};
  std::vector<FrontendGraphModule> modules;
};

struct FrontendClassCapture final {
  std::uint32_t ordinal{};
  std::string module_name;
  std::string name_space;
  std::string class_name;
  FrontendDigest source_sha256{};
  FrontendDigest input_generated_statics_sha256{};
  std::string generated_statics;
  FrontendDigest output_generated_statics_sha256{};
  bool has_statics{};
  std::string compose_onto_class;
};

struct FrontendClassFrame final {
  std::string module_name;
  std::string name_space;
  std::string class_name;
  std::string source;
  std::string generated_statics;
  bool has_statics{};
  std::string compose_onto_class;
};

struct FrontendPreprocessorConfig final {
  bool automatic_imports{};
  bool warn_on_manual_import_statements{};
  bool use_editor_scripts{};
  std::vector<FrontendFlag> effective_flags;
  bool default_function_blueprint_callable{};
  FrontendPropertyEdit default_property_edit_specifier{};
  FrontendPropertyEdit default_property_edit_specifier_for_structs{};
  FrontendPropertyBlueprint default_property_blueprint_specifier{};
  FrontendStaticClassMode static_class_mode{};
  bool script_float_is_float64{};
  bool angelscript_haze{};
  bool enforce_server_rpc_validation{};
  std::vector<std::string> blueprint_event_argument_specializations;
  std::vector<FrontendNativeSuper> native_super_types;
  std::vector<FrontendFNameComparison> fname_comparison_keys;
  bool class_analyze_bound{};
  std::vector<FrontendClassCapture> class_analyze_captures;
  bool process_chunks_bound{};
  std::vector<FrontendGraphCapture> process_chunks_captures;
  bool post_process_code_bound{};
  std::vector<FrontendGraphCapture> post_process_code_captures;
  FrontendDigest canonical_sha256{};
};

struct FrontendClassGeneratorConfig final {
  bool mark_non_uproperty_properties_as_transient{};
  FrontendDigest canonical_sha256{};
};

struct FrontendCompilerOptions final {
  bool error_on_incorrect_editor_only_code{};
  bool warn_on_divergent_comparison_operator_overloads{};
  bool warn_on_implicit_signed_unsigned_conversion{};
  bool warn_on_increment_decrement_in_complex_expression{};
  bool warn_on_unused_return_value_for_const_methods{};
  FrontendDigest canonical_sha256{};
};

enum class FrontendBoundaryKind : std::uint32_t {
  initial_compile_enter = 1,
  precompiled_descriptors_requested = 2,
  preprocessor_constructed = 3,
  initial_compile_return = 4,
};

// Pointer-free payload ready for the existing bridge ABI. The later production wrapper copies
// these fields; it must never derive or substitute a digest itself.
struct FrontendBoundaryProjection final {
  FrontendBoundaryKind kind{FrontendBoundaryKind::initial_compile_enter};
  std::uint32_t observation_rva{};
  std::uint32_t module_count{};
  std::int32_t result_code{};
  FrontendDigest config_sha256{};
  FrontendDigest input_sha256{};
  FrontendDigest output_sha256{};
};

enum class FrontendCallbackKind : std::uint8_t {
  process_chunks = 1,
  post_process_code = 2,
  class_analyze = 3,
};

// Exact generation-specific CALL witnesses. These are validation inputs only: a matching callsite
// does not make the target's mutable Unreal containers pointer-neutral or its delegate mutation
// representable by the portable append-only hook schema.
struct FrontendCallbackCallsite final {
  FrontendCallbackKind kind{};
  std::uint32_t call_rva{};
  std::uint32_t return_rva{};
  std::uint32_t direct_callee_rva{};
  std::uint32_t delegate_storage_rva{};
  std::int32_t relative_displacement{};
  std::array<std::byte, 5> expected_call{};
};

// Both authenticated generations share these object offsets. Their independently verified image
// addresses live in FrontendTargetAddresses below.
namespace frontend_target_layout {
inline constexpr std::size_t manager_settings = 0x4d0;
inline constexpr std::size_t manager_blueprint_specializations = 0x478;
inline constexpr std::size_t settings_preprocessor_flags = 0x28;
inline constexpr std::size_t settings_automatic_imports = 0x39;
inline constexpr std::size_t settings_warn_manual_imports = 0x3a;
inline constexpr std::size_t settings_default_function_blueprint = 0x3c;
inline constexpr std::size_t settings_default_property_edit = 0x3d;
inline constexpr std::size_t settings_default_struct_property_edit = 0x3e;
inline constexpr std::size_t settings_default_property_blueprint = 0x3f;
inline constexpr std::size_t settings_mark_non_uproperty_transient = 0x40;
inline constexpr std::size_t settings_static_class_mode = 0x41;
inline constexpr std::size_t settings_script_float64 = 0x6c;
inline constexpr std::size_t settings_warn_unused_const_return = 0x71;
inline constexpr std::size_t settings_warn_signed_unsigned = 0x72;
inline constexpr std::size_t settings_error_editor_only = 0x73;
inline constexpr std::size_t settings_warn_divergent_comparison = 0x74;
inline constexpr std::size_t settings_warn_complex_increment = 0x75;
inline constexpr std::size_t preprocessor_flags = 0x00;
inline constexpr std::size_t preprocessor_default_function_blueprint = 0x53;
inline constexpr std::size_t preprocessor_default_property_edit = 0x54;
inline constexpr std::size_t preprocessor_default_struct_property_edit = 0x55;
inline constexpr std::size_t preprocessor_default_property_blueprint = 0x56;
inline constexpr std::size_t preprocessor_files = 0x58;
inline constexpr std::size_t preprocessor_file_stride = 0xc8;
inline constexpr std::size_t file_module = 0x00;
inline constexpr std::size_t file_absolute_path = 0x20;
inline constexpr std::size_t file_relative_path = 0x30;
inline constexpr std::size_t file_raw_code = 0x40;
inline constexpr std::size_t file_processed_code = 0x68;
inline constexpr std::size_t file_generated_code = 0x78;
inline constexpr std::size_t module_name = 0x00;
inline constexpr std::size_t module_code = 0x10;
inline constexpr std::size_t module_code_hash = 0x20;
inline constexpr std::size_t code_section_stride = 0x38;
inline constexpr std::size_t code_section_relative_path = 0x00;
inline constexpr std::size_t code_section_absolute_path = 0x10;
inline constexpr std::size_t code_section_code = 0x20;
inline constexpr std::size_t code_section_hash = 0x30;
inline constexpr std::size_t class_name = 0x00;
inline constexpr std::size_t class_code_super = 0x20;
inline constexpr std::size_t class_compose_onto = 0xf8;
inline constexpr std::size_t class_namespace = 0x108;
inline constexpr std::size_t uobject_class = 0x10;
inline constexpr std::size_t uobject_name = 0x18;
inline constexpr std::size_t uobject_outer = 0x20;
inline constexpr std::size_t ustruct_super = 0x40;
inline constexpr std::size_t ustruct_properties_size = 0x58;
inline constexpr std::size_t angelscript_type_get_class_vslot = 0x18;

struct FrontendTargetAddresses final {
  CaptureTargetGeneration generation{};
  std::uint32_t static_names_rva{};
  std::uint32_t use_editor_scripts_rva{};
  std::uint32_t automatic_imports_rva{};
  std::uint32_t fname_pool_rva{};
  std::uint32_t fname_to_string_rva{};
  std::uint32_t type_usage_from_type_id_rva{};
  std::uint32_t process_chunks_call_rva{};
  std::uint32_t process_chunks_return_rva{};
  std::uint32_t process_chunks_delegate_rva{};
  std::uint32_t post_process_code_call_rva{};
  std::uint32_t post_process_code_return_rva{};
  std::uint32_t post_process_code_delegate_rva{};
  std::uint32_t class_analyze_delegate_rva{};
  std::uint32_t class_analyze_call_rva{};
  std::uint32_t class_analyze_return_rva{};
  std::uint32_t multicast_broadcast_rva{};
  std::uint32_t class_analyze_broadcast_rva{};
  std::array<std::uint32_t, 2> class_analyze_accessor_rvas{};
  std::uint32_t initial_compile_enter_rva{};
  std::uint32_t descriptors_requested_rva{};
  std::uint32_t preprocessor_constructed_rva{};
  std::uint32_t initial_compile_return_rva{};
};

inline constexpr FrontendTargetAddresses kFrontendTarget24539464{
    CaptureTargetGeneration::build_24539464,
    0x09d6b2c8,
    0x09d6b341,
    0x09d6b362,
    0x09af8600,
    0x011cf680,
    0x0474d8f0,
    0x0489f822,
    0x0489f827,
    0x09875598,
    0x0489f90c,
    0x0489f911,
    0x098755b0,
    0x098750a8,
    0x0488a237,
    0x0488a23c,
    0x010419b0,
    0x0488a4a0,
    {0x04681b27, 0x04681b60},
    0x04684210,
    0x046842d0,
    0x0468435d,
    0x04685a46,
};

inline constexpr FrontendTargetAddresses kFrontendTarget24878692{
    CaptureTargetGeneration::build_24878692,
    0x09d6c448,
    0x09d6c4c1,
    0x09d6c4e2,
    0x09af9780,
    0x011cf640,
    0x0474d8b0,
    0x0489f7e2,
    0x0489f7e7,
    0x09876598,
    0x0489f8cc,
    0x0489f8d1,
    0x098765b0,
    0x098760a8,
    0x0488a1f7,
    0x0488a1fc,
    0x01041970,
    0x0488a460,
    {0x04681ae7, 0x04681b20},
    0x046841d0,
    0x04684290,
    0x0468431d,
    0x04685a06,
};

inline constexpr const FrontendTargetAddresses& kFrontendTarget =
    kFrontendTarget24878692;
inline constexpr std::uint32_t static_names_rva = kFrontendTarget.static_names_rva;
inline constexpr std::uint32_t use_editor_scripts_rva =
    kFrontendTarget.use_editor_scripts_rva;
inline constexpr std::uint32_t automatic_imports_rva =
    kFrontendTarget.automatic_imports_rva;
inline constexpr std::uint32_t fname_pool_rva = kFrontendTarget.fname_pool_rva;
inline constexpr std::uint32_t fname_to_string_rva = kFrontendTarget.fname_to_string_rva;
inline constexpr std::uint32_t type_usage_from_type_id_rva =
    kFrontendTarget.type_usage_from_type_id_rva;
inline constexpr std::uint32_t process_chunks_call_rva =
    kFrontendTarget.process_chunks_call_rva;
inline constexpr std::uint32_t process_chunks_return_rva =
    kFrontendTarget.process_chunks_return_rva;
inline constexpr std::uint32_t process_chunks_delegate_rva =
    kFrontendTarget.process_chunks_delegate_rva;
inline constexpr std::uint32_t post_process_code_call_rva =
    kFrontendTarget.post_process_code_call_rva;
inline constexpr std::uint32_t post_process_code_return_rva =
    kFrontendTarget.post_process_code_return_rva;
inline constexpr std::uint32_t post_process_code_delegate_rva =
    kFrontendTarget.post_process_code_delegate_rva;
inline constexpr std::uint32_t class_analyze_delegate_rva =
    kFrontendTarget.class_analyze_delegate_rva;
inline constexpr std::uint32_t class_analyze_call_rva =
    kFrontendTarget.class_analyze_call_rva;
inline constexpr std::uint32_t class_analyze_return_rva =
    kFrontendTarget.class_analyze_return_rva;
inline constexpr std::uint32_t multicast_broadcast_rva =
    kFrontendTarget.multicast_broadcast_rva;
inline constexpr std::uint32_t class_analyze_broadcast_rva =
    kFrontendTarget.class_analyze_broadcast_rva;
inline constexpr std::array<FrontendCallbackCallsite, 3> callback_callsites{{
    {FrontendCallbackKind::process_chunks,
     process_chunks_call_rva,
     process_chunks_return_rva,
     multicast_broadcast_rva,
     process_chunks_delegate_rva,
     -59'104'887,
     {std::byte{0xe8}, std::byte{0x89}, std::byte{0x21}, std::byte{0x7a}, std::byte{0xfc}}},
    {FrontendCallbackKind::post_process_code,
     post_process_code_call_rva,
     post_process_code_return_rva,
     multicast_broadcast_rva,
     post_process_code_delegate_rva,
     -59'105'121,
     {std::byte{0xe8}, std::byte{0x9f}, std::byte{0x20}, std::byte{0x7a}, std::byte{0xfc}}},
    {FrontendCallbackKind::class_analyze,
     class_analyze_call_rva,
     class_analyze_return_rva,
     class_analyze_broadcast_rva,
     0,
     612,
     {std::byte{0xe8}, std::byte{0x64}, std::byte{0x02}, std::byte{0x00}, std::byte{0x00}}},
}};
static_assert(process_chunks_call_rva + 5 == process_chunks_return_rva);
static_assert(post_process_code_call_rva + 5 == post_process_code_return_rva);
static_assert(class_analyze_call_rva + 5 == class_analyze_return_rva);
static_assert(
    static_cast<std::int64_t>(process_chunks_return_rva) - 59'104'887 ==
    multicast_broadcast_rva);
static_assert(
    static_cast<std::int64_t>(post_process_code_return_rva) - 59'105'121 ==
    multicast_broadcast_rva);
static_assert(class_analyze_return_rva + 612 == class_analyze_broadcast_rva);
inline constexpr std::uint32_t initial_compile_enter_rva =
    kFrontendTarget.initial_compile_enter_rva;
inline constexpr std::uint32_t descriptors_requested_rva =
    kFrontendTarget.descriptors_requested_rva;
inline constexpr std::uint32_t preprocessor_constructed_rva =
    kFrontendTarget.preprocessor_constructed_rva;
inline constexpr std::uint32_t initial_compile_return_rva =
    kFrontendTarget.initial_compile_return_rva;
static_assert(kFrontendTarget.generation == kCaptureTarget.generation);
}  // namespace frontend_target_layout

FrontendObserverError project_frontend_settings_v1(
    const std::byte* settings,
    std::size_t settings_bytes,
    const std::byte* preprocessor,
    std::size_t preprocessor_bytes,
    bool automatic_imports,
    bool use_editor_scripts,
    FrontendPreprocessorConfig& preprocessor_config,
    FrontendClassGeneratorConfig& class_generator_config,
    FrontendCompilerOptions& compiler_options) noexcept;

FrontendObserverError derive_native_super_v1(
    const FrontendNativeClassWitness& witness,
    FrontendNativeSuper& projection) noexcept;

FrontendObserverError make_fname_comparison_key_v1(
    std::string_view spelling,
    std::uint32_t target_comparison_index,
    FrontendFNameComparison& projection) noexcept;

class FrontendSemanticObserver final {
 public:
  FrontendObserverError set_hook_bindings(
      bool class_analyze,
      bool process_chunks,
      bool post_process_code) noexcept;
  FrontendObserverError begin_class_analyze(const FrontendClassFrame& frame) noexcept;
  FrontendObserverError complete_class_analyze(const FrontendClassFrame& frame) noexcept;
  FrontendObserverError begin_process_chunks(
      const std::vector<FrontendGraphModule>& modules) noexcept;
  FrontendObserverError complete_process_chunks(
      const std::vector<FrontendGraphModule>& modules) noexcept;
  FrontendObserverError begin_post_process_code(
      const std::vector<FrontendGraphModule>& modules) noexcept;
  FrontendObserverError complete_post_process_code(
      const std::vector<FrontendGraphModule>& modules) noexcept;
  FrontendObserverError finish(FrontendPreprocessorConfig& config) noexcept;

 private:
  enum class PendingKind : std::uint8_t { none, class_analyze, process_chunks, post_process };
  FrontendObserverError begin_graph(
      PendingKind kind,
      const std::vector<FrontendGraphModule>& modules) noexcept;
  FrontendObserverError complete_graph(
      PendingKind kind,
      const std::vector<FrontendGraphModule>& modules,
      std::vector<FrontendGraphCapture>& captures) noexcept;
  PendingKind pending_{PendingKind::none};
  FrontendClassFrame pending_class_;
  std::vector<FrontendGraphModule> pending_modules_;
  FrontendDigest pending_graph_digest_{};
  std::vector<FrontendClassCapture> class_captures_;
  std::vector<FrontendGraphCapture> process_captures_;
  std::vector<FrontendGraphCapture> post_captures_;
  bool class_observed_{};
  bool process_observed_{};
  bool post_observed_{};
  bool bindings_set_{};
  bool class_bound_{};
  bool process_bound_{};
  bool post_bound_{};
  bool finished_{};
};

FrontendObserverError serialize_preprocessor_config_json_v1(
    FrontendPreprocessorConfig& config,
    std::string& json) noexcept;
FrontendObserverError serialize_class_generator_config_json_v1(
    FrontendClassGeneratorConfig& config,
    std::string& json) noexcept;
FrontendObserverError serialize_compiler_options_json_v1(
    FrontendCompilerOptions& options,
    std::string& json) noexcept;

FrontendObserverError frontend_config_set_digest_v1(
    const FrontendPreprocessorConfig& preprocessor,
    const FrontendClassGeneratorConfig& class_generator,
    const FrontendCompilerOptions& compiler_options,
    FrontendDigest& digest) noexcept;

FrontendObserverError project_initial_compile_enter_v1(
    const FrontendDigest& config_sha256,
    FrontendBoundaryProjection& boundary) noexcept;
FrontendObserverError project_precompiled_descriptors_v1(
    const FrontendDigest& config_sha256,
    const std::vector<FrontendGraphModule>& modules,
    FrontendBoundaryProjection& boundary) noexcept;
FrontendObserverError project_preprocessor_constructed_v1(
    const FrontendDigest& config_sha256,
    FrontendBoundaryProjection& boundary) noexcept;
FrontendObserverError project_initial_compile_return_v1(
    const FrontendDigest& config_sha256,
    const std::vector<FrontendGraphModule>& modules,
    FrontendBoundaryProjection& boundary) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_frontend_observer_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
