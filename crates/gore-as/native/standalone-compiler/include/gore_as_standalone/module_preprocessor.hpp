#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace gore::as::standalone {

inline constexpr std::size_t max_preprocessor_sources = 4'096U;
inline constexpr std::size_t max_preprocessor_flags = 4'096U;
inline constexpr std::size_t max_preprocessor_path_bytes = 4'096U;
inline constexpr std::size_t max_preprocessor_source_bytes = 16U * 1024U * 1024U;
inline constexpr std::size_t max_preprocessor_total_source_bytes = 256U * 1024U * 1024U;
inline constexpr std::size_t max_preprocessor_imports = 1'000'000U;
inline constexpr std::size_t max_preprocessor_static_names = 1'000'000U;
inline constexpr std::size_t max_preprocessor_post_init_functions = 1'000'000U;
inline constexpr std::size_t max_preprocessor_base_modules = 100'000U;
inline constexpr std::size_t max_preprocessor_base_classes = 1'000'000U;
inline constexpr std::size_t max_preprocessor_external_generated_bytes =
    16U * 1024U * 1024U;
inline constexpr std::size_t max_preprocessor_hook_detail_bytes = 4'096U;

enum class preprocessor_diagnostic_severity { warning, error };

struct preprocessor_diagnostic {
    preprocessor_diagnostic_severity severity = preprocessor_diagnostic_severity::error;
    std::string absolute_path;
    std::uint32_t row = 1U;
    std::uint32_t column = 1U;
    std::string message;
};

struct preprocessor_flag {
    std::string name;
    bool value = false;
};

enum class static_class_mode { allowed, deprecated, disallowed };
enum class property_edit_specifier {
    edit_anywhere,
    edit_instance_only,
    edit_defaults_only,
    not_editable,
};
enum class property_blueprint_specifier {
    blueprint_read_write,
    blueprint_read_only,
    blueprint_hidden,
};

enum class native_super_kind {
    actor,
    actor_component,
    engine_subsystem,
    editor_subsystem,
    game_instance_subsystem,
    world_subsystem,
    local_player_subsystem,
    other_uobject,
};

struct native_super_type {
    std::string angelscript_type_name;
    std::string unreal_class_path;
    std::uint64_t property_offset = 0U;
    native_super_kind kind = native_super_kind::other_uobject;
    bool cannot_derive_angelscript = false;
};

// FName comparison is not Unicode case folding. Non-ASCII spellings therefore
// require a captured comparison identity from the target build. Spellings
// which compare equal carry the same opaque key. ASCII retains the donor's
// locale-independent folding without requiring profile entries.
struct fname_comparison_key {
    std::string spelling;
    std::string key;
};

struct preprocessor_metadata {
    std::string name;
    std::string value;
    std::int32_t subject_index = -1;
};

struct preprocessed_property_description {
    std::string property_name;
    std::string literal_type;
    std::uint32_t line = 1U;
    bool blueprint_readable = false;
    bool blueprint_writable = false;
    bool editable_on_defaults = false;
    bool editable_on_instance = false;
    bool edit_const = false;
    bool instanced_reference = false;
    bool persistent_instance = false;
    bool advanced_display = false;
    bool transient = false;
    bool replicated = false;
    std::int32_t replication_condition = 0;
    bool skip_replication = false;
    bool skip_serialization = false;
    bool save_game = false;
    bool rep_notify = false;
    bool config = false;
    bool interp = false;
    bool asset_registry_searchable = false;
    bool no_clear = false;
    std::vector<preprocessor_metadata> metadata;
};

struct preprocessed_function_description {
    std::string function_name;
    std::string script_function_name;
    std::uint32_t line = 1U;
    bool blueprint_callable = false;
    bool blueprint_override = false;
    bool blueprint_event = false;
    bool blueprint_pure = false;
    bool net_function = false;
    bool net_multicast = false;
    bool net_client = false;
    bool net_server = false;
    bool net_validate = false;
    bool unreliable = false;
    bool blueprint_authority_only = false;
    bool exec = false;
    bool dev_function = false;
    bool can_override_event = true;
    bool is_static = false;
    bool thread_safe = false;
    std::vector<preprocessor_metadata> metadata;
};

struct preprocessed_class_description {
    std::string class_name;
    std::string name_space;
    std::string super_class;
    std::string static_class_global_variable_name;
    std::string defaults_code;
    std::uint32_t line = 1U;
    bool is_struct = false;
    bool abstract = false;
    bool transient = false;
    bool hide_dropdown = false;
    bool default_to_instanced = false;
    bool edit_inline_new = false;
    bool deprecated = false;
    bool placeable = true;
    bool is_statics_class = false;
    bool super_is_code_class = false;
    std::string code_super_class;
    native_super_kind code_super_kind = native_super_kind::other_uobject;
    std::string config_name;
    std::string compose_onto_class;
    std::vector<preprocessor_metadata> metadata;
    std::vector<preprocessed_property_description> properties;
    std::vector<preprocessed_function_description> methods;
};

struct preprocessed_enum_description {
    std::string enum_name;
    std::string name_space;
    std::uint32_t line = 1U;
    std::vector<preprocessor_metadata> metadata;
};

struct preprocessed_delegate_description {
    std::string delegate_name;
    std::string name_space;
    std::uint32_t line = 1U;
    bool multicast = false;
};

struct preprocessor_options {
    // Mirrors FAngelscriptManager::bUseAutomaticImportMethod. With automatic
    // imports enabled, the donor does not sort, blank, or publish manual module
    // imports during preprocessing.
    bool automatic_imports = true;
    static_class_mode static_classes = static_class_mode::allowed;
    bool default_function_blueprint_callable = false;
    property_edit_specifier default_property_edit =
        property_edit_specifier::edit_anywhere;
    property_edit_specifier default_struct_property_edit =
        property_edit_specifier::edit_anywhere;
    property_blueprint_specifier default_property_blueprint =
        property_blueprint_specifier::blueprint_read_write;
    bool script_float_is_float64 = false;
    bool angelscript_haze = false;
    bool enforce_server_rpc_validation = false;
    std::vector<std::string> blueprint_event_argument_specializations;
    std::vector<native_super_type> native_super_types;
    std::vector<preprocessor_flag> flags;
    std::vector<fname_comparison_key> fname_comparison_keys;

    // GenerateStaticName appends to the manager-global FName table. A full
    // regeneration seeds this with the decoded pristine cache tail and emits
    // the resulting table back through lexical_preprocess_result.
    std::vector<std::string> static_names;
};

struct preprocessor_source {
    std::string relative_path;
    std::string absolute_path;
    std::string code;
    enum class operation { add, edit } overlay_operation = operation::add;
    // Empty preserves the donor's filename-derived module identity. Protocol
    // callers provide this field and it must match the derived identity.
    std::string module_name;
};

// Authoritative class ancestry retained by the decoded pristine cache. Base
// modules replaced by an `edit` overlay are excluded before hierarchy lookup.
struct preprocessor_base_class {
    std::string class_name;
    std::string name_space;
    std::string super_class;
    std::string code_super_class;
    bool super_is_code_class = false;
    bool is_struct = false;
};

struct preprocessor_base_module {
    std::string module_name;
    std::vector<preprocessor_base_class> classes;
};

struct preprocessed_code_section {
    std::string relative_path;
    std::string absolute_path;
    std::string conditioned_code;
    std::int64_t code_hash = 0;
};

struct editor_only_line_block {
    std::uint32_t first_line = 1U;
    std::uint32_t last_line = 1U;
};

// UE-free projection of the source-bearing parts of FAngelscriptModuleDesc.
// The historical `lexical_` name is retained for source compatibility while
// the record now includes reflection and generated-code descriptors.
struct lexical_module_description {
    std::string module_name;
    std::int64_t code_hash = 0;
    std::vector<preprocessed_code_section> code;
    std::vector<std::string> imported_modules;
    std::vector<std::string> post_init_functions;
    std::vector<preprocessed_class_description> classes;
    std::vector<preprocessed_enum_description> enums;
    std::vector<preprocessed_delegate_description> delegates;
    std::string statics_class_name;
    std::vector<editor_only_line_block> editor_only_blocks;
};

struct lexical_preprocess_result {
    bool ok = false;
    std::vector<lexical_module_description> modules;
    std::vector<std::string> static_names;
    std::vector<preprocessor_diagnostic> diagnostics;
};

// UE-free equivalents of the donor's three external preprocessing extension
// points. The graph hooks receive read-only descriptors and may only append
// generated declarations, which preserves source offsets used by the core
// preprocessing passes. ClassAnalyze retains the donor's mutable class record,
// GeneratedStatics string and bHasStatics result, including ComposeOnto.
struct preprocessor_graph_hook_module {
    const lexical_module_description* module = nullptr;
    std::string generated_declarations;
};

struct preprocessor_hooks {
    void* context = nullptr;
    bool (*class_analyze)(
        void* context,
        const preprocessor_source& source,
        preprocessed_class_description& description,
        std::string& generated_statics,
        bool& has_statics,
        std::string& detail) noexcept = nullptr;
    bool (*process_chunks)(
        void* context,
        preprocessor_graph_hook_module* modules,
        std::size_t module_count,
        std::string& detail) noexcept = nullptr;
    bool (*post_process_code)(
        void* context,
        preprocessor_graph_hook_module* modules,
        std::size_t module_count,
        std::string& detail) noexcept = nullptr;
};

// Exact donor CodeHash: XXH64(seed 0) over the processed FString's UTF-16LE
// code units. Empty processed code uses the donor's sentinel hash 0.
bool compute_processed_code_hash_utf8(
    const std::string& processed_code,
    std::int64_t& code_hash);

// Source transformations and descriptors are ported from the pinned donor.
// Unreal reflection inputs are supplied through the sealed native-super and
// base-cache projections; missing profile facts reject instead of being
// inferred from the host machine.
lexical_preprocess_result preprocess_lexical_module_graph(
    const preprocessor_options& options,
    const std::vector<preprocessor_source>& sources,
    const std::vector<preprocessor_base_module>& base_modules = {},
    const preprocessor_hooks* hooks = nullptr);

} // namespace gore::as::standalone
