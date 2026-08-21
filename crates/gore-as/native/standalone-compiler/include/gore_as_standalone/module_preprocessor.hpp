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

struct preprocessor_options {
    // Mirrors FAngelscriptManager::bUseAutomaticImportMethod. With automatic
    // imports enabled, the donor does not sort, blank, or publish manual module
    // imports during preprocessing.
    bool automatic_imports = true;
    std::vector<preprocessor_flag> flags;
};

struct preprocessor_source {
    std::string relative_path;
    std::string absolute_path;
    std::string code;
};

struct preprocessed_code_section {
    std::string relative_path;
    std::string absolute_path;
    std::string conditioned_code;
};

// This is the lexical/module-graph portion of FAngelscriptModuleDesc. Class,
// enum, delegate, macro, defaults, literal and generated-code descriptors are
// deliberately not represented until their exact donor phases are ported.
struct lexical_module_description {
    std::string module_name;
    std::vector<preprocessed_code_section> code;
    std::vector<std::string> imported_modules;
};

struct lexical_preprocess_result {
    bool ok = false;
    std::vector<lexical_module_description> modules;
    std::vector<preprocessor_diagnostic> diagnostics;
};

// Exact lexical front of the pinned FAngelscriptPreprocessor: bounded source
// validation, conditional blanking, top-level module-import discovery and the
// explicit-import dependency order. It intentionally stops before chunk
// analysis/macros and therefore is not yet a complete source-to-module API.
lexical_preprocess_result preprocess_lexical_module_graph(
    const preprocessor_options& options,
    const std::vector<preprocessor_source>& sources);

} // namespace gore::as::standalone
