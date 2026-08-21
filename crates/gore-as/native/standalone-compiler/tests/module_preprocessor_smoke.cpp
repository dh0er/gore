#include "gore_as_standalone/module_preprocessor.hpp"

#include <iostream>
#include <string>
#include <vector>

namespace standalone = gore::as::standalone;

namespace {

int fail(const char* message) {
    std::cerr << message << '\n';
    return 1;
}

standalone::preprocessor_source source(
    std::string relative_path,
    std::string code) {
    return {
        relative_path,
        "C:/sealed/Script/" + relative_path,
        std::move(code)};
}

bool is_blank_except_whitespace(const std::string& value) {
    for (const char character : value) {
        if (character != ' ' && character != '\t' &&
            character != '\r' && character != '\n') return false;
    }
    return true;
}

} // namespace

int main() {
    const auto empty = standalone::preprocess_lexical_module_graph({}, {});
    if (!empty.ok || !empty.modules.empty() || !empty.diagnostics.empty()) {
        return fail("empty donor source set did not remain a successful no-op");
    }

    standalone::preprocessor_options explicit_imports;
    explicit_imports.automatic_imports = false;
    explicit_imports.flags = {
        {"DEFINED_FALSE", false},
        {"ENABLED", true},
        {"DISABLED", false},
    };

    std::vector<standalone::preprocessor_source> sources;
    sources.push_back(source("Game/Consumer.as", R"AS(import Game.Provider;
import void ImportedCall() from "Native";
namespace Outer
{
    import Game.NamespaceProvider;
}
#ifdef DEFINED_FALSE
int PresentBecauseDefined() { return 1; }
#endif
#if DISABLED
int RemovedA() { return 0; }
#elif ENABLED
int Selected() { return 42; }
#else
int RemovedB() { return 0; }
#endif
#ifndef MISSING
int MissingIsAbsent() { return 1; }
#endif
const string Literal = "import Not.A.Module;";
// import Not.A.Comment;
)AS"));
    sources.push_back(source(
        "Game/Provider.as", "int Provider() { return 20; }\n"));
    sources.push_back(source(
        "Game/NamespaceProvider.as", "int NamespaceProvider() { return 22; }\n"));

    const auto explicit_result =
        standalone::preprocess_lexical_module_graph(explicit_imports, sources);
    if (!explicit_result.ok || !explicit_result.diagnostics.empty()) {
        return fail("explicit-import lexical preprocessing failed");
    }
    if (explicit_result.modules.size() != 3U ||
        explicit_result.modules[0].module_name != "Game.Provider" ||
        explicit_result.modules[1].module_name != "Game.NamespaceProvider" ||
        explicit_result.modules[2].module_name != "Game.Consumer") {
        return fail("explicit imports did not produce donor dependency order");
    }
    const auto& consumer = explicit_result.modules[2];
    if (consumer.imported_modules !=
        std::vector<std::string>{"Game.Provider", "Game.NamespaceProvider"}) {
        return fail("top-level module imports were not discovered exactly");
    }
    const std::string& conditioned = consumer.code[0].conditioned_code;
    if (conditioned.find("PresentBecauseDefined") == std::string::npos ||
        conditioned.find("Selected") == std::string::npos ||
        conditioned.find("MissingIsAbsent") == std::string::npos ||
        conditioned.find("RemovedA") != std::string::npos ||
        conditioned.find("RemovedB") != std::string::npos ||
        conditioned.find("import void ImportedCall()") == std::string::npos ||
        conditioned.find("import Not.A.Module;") == std::string::npos) {
        return fail("conditionals, function imports, strings or comments drifted");
    }
    const std::size_t first_line_end = conditioned.find('\n');
    if (first_line_end == std::string::npos ||
        !is_blank_except_whitespace(conditioned.substr(0U, first_line_end))) {
        return fail("manual module import was not blanked with layout preservation");
    }

    standalone::preprocessor_options automatic = explicit_imports;
    automatic.automatic_imports = true;
    const auto automatic_result =
        standalone::preprocess_lexical_module_graph(automatic, sources);
    if (!automatic_result.ok || automatic_result.modules.size() != 3U ||
        automatic_result.modules[0].module_name != "Game.Consumer" ||
        !automatic_result.modules[0].imported_modules.empty() ||
        automatic_result.modules[0].code[0].conditioned_code.find(
            "import Game.Provider;") != 0U) {
        return fail("automatic-import mode did not preserve donor input behavior");
    }

    const auto unknown = standalone::preprocess_lexical_module_graph(
        automatic,
        {source("Bad/Unknown.as", "#if UNKNOWN\nint X;\n#endif\n")});
    if (unknown.ok || unknown.diagnostics.size() != 1U ||
        unknown.diagnostics[0].row != 1U ||
        unknown.diagnostics[0].message != "Invalid preprocessor condition: UNKNOWN") {
        return fail("unknown preprocessor flag did not fail with exact diagnostics");
    }

    const auto cycle = standalone::preprocess_lexical_module_graph(
        explicit_imports,
        {source("Cycle/A.as", "import Cycle.B;\n"),
         source("Cycle/B.as", "import Cycle.A;\n")});
    if (cycle.ok || cycle.modules.size() != 2U || cycle.diagnostics.size() != 3U ||
        cycle.diagnostics[0].message !=
            "Detected circular import of module Cycle.A. Import chain:" ||
        cycle.diagnostics[1].message != "   => Cycle.B" ||
        cycle.diagnostics[2].message != "   => Cycle.A") {
        return fail("circular import diagnostics or recovery order drifted");
    }

    const auto unclosed = standalone::preprocess_lexical_module_graph(
        automatic,
        {source("Bad/Unclosed.as", "#if ENABLED\nint X;\n")});
    if (unclosed.ok || unclosed.diagnostics.size() != 1U ||
        unclosed.diagnostics[0].row != 3U ||
        unclosed.diagnostics[0].message !=
            "Preceding preprocessor #if/#ifdef/#else was not closed, missing #endif.") {
        return fail("unclosed conditional diagnostic drifted");
    }

    std::cout << "G1R lexical preprocessor smoke covered conditionals, imports and cycles\n";
    return 0;
}
