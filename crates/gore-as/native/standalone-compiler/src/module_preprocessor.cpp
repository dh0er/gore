#include "gore_as_standalone/module_preprocessor.hpp"

#include <algorithm>
#include <limits>
#include <string_view>
#include <unordered_map>
#include <unordered_set>

namespace gore::as::standalone {
namespace {

struct import_description {
    std::string module_name;
    std::size_t start = 0U;
    std::size_t end = 0U;
    std::uint32_t line = 1U;
};

struct source_state {
    lexical_module_description module;
    std::vector<import_description> imports;
    bool imports_resolved = false;
    bool resolving_imports = false;
};

struct active_ifdef {
    bool value = false;
    bool any_branch_taken = false;
    bool has_else = false;
    std::string condition;
};

bool is_whitespace(const char value) noexcept {
    return value == '\n' || value == '\t' || value == ' ' || value == '\r';
}

bool is_control(const unsigned char value) noexcept {
    return value < 0x20U || value == 0x7fU;
}

std::string trim(std::string_view value) {
    while (!value.empty() && is_whitespace(value.front())) value.remove_prefix(1U);
    while (!value.empty() && is_whitespace(value.back())) value.remove_suffix(1U);
    return std::string(value);
}

bool starts_at(
    const std::string& value,
    const std::size_t position,
    const std::string_view prefix) noexcept {
    return position <= value.size() && prefix.size() <= value.size() - position &&
        value.compare(position, prefix.size(), prefix) == 0;
}

std::string replace_all(
    std::string value,
    const std::string_view needle,
    const std::string_view replacement) {
    std::size_t position = 0U;
    while ((position = value.find(needle, position)) != std::string::npos) {
        value.replace(position, needle.size(), replacement);
        position += replacement.size();
    }
    return value;
}

std::string filename_to_module_name(const std::string& filename) {
    return replace_all(replace_all(filename, ".as", ""), "/", ".");
}

std::string read_identifier(const std::string& code, const std::size_t position) {
    std::size_t end = position;
    while (end < code.size() && code[end] != '\n' && code[end] != '/' && code[end] != '{') {
        ++end;
    }
    return trim(std::string_view(code).substr(position, end - position));
}

void kill_raw_line(std::string& code, const std::size_t position) {
    for (std::size_t end = position;
         end < code.size() && code[end] != '\n' && code[end] != '/';
         ++end) {
        code[end] = ' ';
    }
}

void replace_with_blank(std::string& code, const std::size_t start, const std::size_t end) {
    if (start >= code.size() || end == 0U || end > code.size() || start >= end) return;
    for (std::size_t position = start; position < end; ++position) {
        if (!is_whitespace(code[position])) code[position] = ' ';
    }
}

void add_diagnostic(
    lexical_preprocess_result& result,
    const preprocessor_source& source,
    const std::uint32_t line,
    std::string message) {
    result.diagnostics.push_back({
        preprocessor_diagnostic_severity::error,
        source.absolute_path,
        line,
        1U,
        std::move(message)});
}

bool valid_text_field(const std::string& value, const bool allow_empty) noexcept {
    if ((!allow_empty && value.empty()) || value.find('\0') != std::string::npos) return false;
    return std::none_of(value.begin(), value.end(), [](const char raw) {
        const auto value = static_cast<unsigned char>(raw);
        return is_control(value);
    });
}

bool validate_inputs(
    const preprocessor_options& options,
    const std::vector<preprocessor_source>& sources,
    lexical_preprocess_result& result) {
    if (sources.size() > max_preprocessor_sources) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "preprocessor source count exceeds the bounded maximum"});
        return false;
    }
    if (options.flags.size() > max_preprocessor_flags) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "preprocessor flag count exceeds the bounded maximum"});
        return false;
    }

    std::unordered_set<std::string> flag_names;
    flag_names.reserve(options.flags.size());
    for (const preprocessor_flag& flag : options.flags) {
        if (!valid_text_field(flag.name, false) || !flag_names.insert(flag.name).second) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "preprocessor flags contain an invalid or duplicate name"});
            return false;
        }
    }

    std::size_t total_source_bytes = 0U;
    for (const preprocessor_source& source : sources) {
        if (!valid_text_field(source.relative_path, false) ||
            !valid_text_field(source.absolute_path, false) ||
            source.relative_path.size() > max_preprocessor_path_bytes ||
            source.absolute_path.size() > max_preprocessor_path_bytes) {
            add_diagnostic(result, source, 1U, "source path is invalid or too long");
            return false;
        }
        if (source.code.size() > max_preprocessor_source_bytes) {
            add_diagnostic(result, source, 1U, "source exceeds the per-file byte limit");
            return false;
        }
        if (source.code.find('\0') != std::string::npos) {
            add_diagnostic(result, source, 1U, "source contains an embedded NUL byte");
            return false;
        }
        if (total_source_bytes > max_preprocessor_total_source_bytes - source.code.size()) {
            add_diagnostic(result, source, 1U, "source set exceeds the total byte limit");
            return false;
        }
        total_source_bytes += source.code.size();
        if (filename_to_module_name(source.relative_path).empty()) {
            add_diagnostic(result, source, 1U, "source path produces an empty module name");
            return false;
        }
    }
    return true;
}

bool is_start_of_identifier(const std::string& code, const std::size_t position) noexcept {
    if (position == 0U) return true;
    const char previous = code[position - 1U];
    // Preserve the pinned donor's exact range check (0..1, not 0..9).
    return !((previous >= 'a' && previous <= 'z') ||
             (previous >= 'A' && previous <= 'Z') ||
             (previous >= '0' && previous <= '1') || previous == '_');
}

void scan_source(
    const preprocessor_options& options,
    const std::unordered_map<std::string, bool>& flags,
    const preprocessor_source& source,
    source_state& state,
    lexical_preprocess_result& result,
    std::size_t& total_imports) {
    std::string& code = state.module.code.front().conditioned_code;
    std::vector<active_ifdef> ifdefs;
    std::vector<std::string> namespace_stack;
    bool ifdef_stack_is_false = false;
    bool in_comment = false;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::size_t scope_count = 0U;
    std::uint32_t line_number = 1U;

    const auto update_ifdef_stack = [&]() {
        ifdef_stack_is_false = std::any_of(ifdefs.begin(), ifdefs.end(),
            [](const active_ifdef& active) { return !active.value; });
    };
    const auto parse_condition = [&](const std::string& expression) {
        std::string lookup = expression;
        bool negate = false;
        if (!lookup.empty() && lookup.front() == '!') {
            negate = true;
            lookup.erase(lookup.begin());
        }
        const auto found = flags.find(lookup);
        if (found == flags.end()) {
            add_diagnostic(
                result, source, line_number,
                "Invalid preprocessor condition: " + expression);
            return false;
        }
        return found->second != negate;
    };
    const auto top_level_scope = [&]() { return scope_count <= namespace_stack.size(); };

    for (std::size_t position = 0U; position < code.size(); ++position) {
        char current = code[position];

        if (current == '#' && !in_comment) {
            if (starts_at(code, position, "#ifdef ")) {
                const std::string identifier = read_identifier(code, position + 7U);
                const bool value = flags.find(identifier) != flags.end();
                kill_raw_line(code, position);
                ifdefs.push_back({value, value, false, identifier});
                update_ifdef_stack();
            } else if (starts_at(code, position, "#ifndef ")) {
                const std::string identifier = read_identifier(code, position + 8U);
                const bool value = flags.find(identifier) == flags.end();
                kill_raw_line(code, position);
                ifdefs.push_back({value, value, false, identifier});
                update_ifdef_stack();
            } else if (starts_at(code, position, "#if ")) {
                const std::string expression = read_identifier(code, position + 4U);
                const bool value = parse_condition(expression);
                kill_raw_line(code, position);
                ifdefs.push_back({value, value, false, expression});
                update_ifdef_stack();
            } else if (starts_at(code, position, "#elif ")) {
                const std::string expression = read_identifier(code, position + 6U);
                kill_raw_line(code, position);
                if (ifdefs.empty() || ifdefs.back().has_else) {
                    add_diagnostic(
                        result, source, line_number,
                        "Invalid #elif, no matching #if found.");
                } else {
                    const bool value = parse_condition(expression);
                    active_ifdef& active = ifdefs.back();
                    active.condition = expression;
                    if (active.any_branch_taken) {
                        active.value = false;
                    } else {
                        active.value = value;
                        if (value) active.any_branch_taken = true;
                    }
                }
                update_ifdef_stack();
            } else if (starts_at(code, position, "#else")) {
                kill_raw_line(code, position);
                if (ifdefs.empty() || ifdefs.back().has_else) {
                    add_diagnostic(
                        result, source, line_number,
                        "Invalid #else, no matching #if found.");
                } else {
                    active_ifdef& active = ifdefs.back();
                    active.value = !active.any_branch_taken;
                    active.any_branch_taken = true;
                    active.has_else = true;
                    if (!active.condition.empty() && active.condition.front() == '!') {
                        active.condition.erase(active.condition.begin());
                    } else {
                        active.condition.insert(active.condition.begin(), '!');
                    }
                }
                update_ifdef_stack();
            } else if (starts_at(code, position, "#endif")) {
                kill_raw_line(code, position);
                if (ifdefs.empty()) {
                    add_diagnostic(
                        result, source, line_number,
                        "Invalid #endif, no matching #if found.");
                } else {
                    ifdefs.pop_back();
                }
                update_ifdef_stack();
            } else if (starts_at(code, position, "#restrict usage allow ") ||
                       starts_at(code, position, "#restrict usage disallow ")) {
                // Shipping builds discard editor-only usage restrictions but
                // still blank their complete directive line.
                kill_raw_line(code, position);
            }
        }

        if (ifdef_stack_is_false && !is_whitespace(current)) {
            current = ' ';
            code[position] = ' ';
        }

        switch (current) {
        case 'n':
            if (starts_at(code, position, "namespace") &&
                position + 9U < code.size() && is_whitespace(code[position + 9U]) &&
                top_level_scope() && !in_string && !in_comment) {
                namespace_stack.push_back(read_identifier(code, position + 10U));
            }
            break;
        case 'i':
            if (starts_at(code, position, "import") &&
                position + 6U < code.size() && is_whitespace(code[position + 6U]) &&
                top_level_scope() && !in_string && !in_comment &&
                is_start_of_identifier(code, position)) {
                const std::size_t module_start = position + 7U;
                std::size_t module_end = module_start;
                while (module_end < code.size() && code[module_end] != ';') ++module_end;
                const std::string module_name = trim(
                    std::string_view(code).substr(module_start, module_end - module_start));
                if (module_name.find('(') == std::string::npos) {
                    if (total_imports >= max_preprocessor_imports) {
                        add_diagnostic(
                            result, source, line_number,
                            "module import count exceeds the bounded maximum");
                    } else {
                        ++total_imports;
                        state.imports.push_back({module_name, position, module_end, line_number});
                    }
                }
            }
            break;
        case '{':
            if (!in_string && !in_comment) ++scope_count;
            break;
        case '}':
            if (!in_string && !in_comment) {
                if (top_level_scope() && !namespace_stack.empty()) namespace_stack.pop_back();
                if (scope_count > 0U) --scope_count;
            }
            break;
        case '/':
            if (position + 1U < code.size() && !in_comment && !in_string) {
                const char next = code[position + 1U];
                if (next == '/') {
                    in_line_comment = true;
                    in_comment = true;
                    ++position;
                } else if (next == '*') {
                    in_block_comment = true;
                    in_comment = true;
                    ++position;
                }
            }
            break;
        case '*':
            if (position + 1U < code.size() && in_block_comment && code[position + 1U] == '/') {
                in_block_comment = false;
                in_comment = false;
                ++position;
            }
            break;
        case '"':
            if (!in_comment) {
                bool escaped = false;
                if (in_string) {
                    std::size_t check = position;
                    while (check > 0U && code[check - 1U] == '\\') {
                        escaped = !escaped;
                        --check;
                    }
                }
                if (!escaped) in_string = !in_string;
            }
            break;
        case '\n':
            if (in_line_comment) {
                in_line_comment = false;
                in_comment = false;
            }
            if (line_number != (std::numeric_limits<std::uint32_t>::max)()) ++line_number;
            break;
        default:
            break;
        }
    }

    if (!ifdefs.empty()) {
        add_diagnostic(
            result, source, line_number,
            "Preceding preprocessor #if/#ifdef/#else was not closed, missing #endif.");
    }

    (void)options;
}

void process_imports(
    const std::size_t index,
    const std::vector<preprocessor_source>& sources,
    std::vector<source_state>& states,
    std::vector<std::size_t>& sorted,
    std::vector<std::size_t>& chain,
    lexical_preprocess_result& result) {
    source_state& state = states[index];
    if (state.imports_resolved) return;
    if (state.resolving_imports) {
        add_diagnostic(
            result, sources[index], 1U,
            "Detected circular import of module " + state.module.module_name +
                ". Import chain:");
        for (auto previous = chain.rbegin(); previous != chain.rend(); ++previous) {
            add_diagnostic(
                result, sources[index], 1U,
                "   => " + states[*previous].module.module_name);
        }
        return;
    }

    state.resolving_imports = true;
    chain.push_back(index);
    for (const import_description& import : state.imports) {
        const auto found = std::find_if(states.begin(), states.end(),
            [&import](const source_state& candidate) {
                return candidate.module.module_name == import.module_name;
            });
        if (found != states.end()) {
            const auto dependency = static_cast<std::size_t>(std::distance(states.begin(), found));
            process_imports(dependency, sources, states, sorted, chain, result);
        }
        state.module.imported_modules.push_back(import.module_name);
        // End is the semicolon position; the donor requests End+1 and the
        // replacement helper silently does nothing if that range is invalid.
        if (import.end < state.module.code.front().conditioned_code.size()) {
            replace_with_blank(
                state.module.code.front().conditioned_code, import.start, import.end + 1U);
        }
    }
    chain.pop_back();
    state.imports_resolved = true;
    state.resolving_imports = false;
    sorted.push_back(index);
}

} // namespace

lexical_preprocess_result preprocess_lexical_module_graph(
    const preprocessor_options& options,
    const std::vector<preprocessor_source>& sources) {
    lexical_preprocess_result result;
    if (!validate_inputs(options, sources, result)) return result;

    std::unordered_map<std::string, bool> flags;
    flags.reserve(options.flags.size());
    for (const preprocessor_flag& flag : options.flags) flags.emplace(flag.name, flag.value);

    std::vector<source_state> states;
    states.reserve(sources.size());
    std::size_t total_imports = 0U;
    for (const preprocessor_source& source : sources) {
        source_state state;
        state.module.module_name = filename_to_module_name(source.relative_path);
        state.module.code.push_back({source.relative_path, source.absolute_path, source.code});
        states.push_back(std::move(state));
        scan_source(options, flags, source, states.back(), result, total_imports);
    }

    std::vector<std::size_t> order;
    order.reserve(states.size());
    if (options.automatic_imports) {
        for (std::size_t index = 0U; index < states.size(); ++index) order.push_back(index);
    } else {
        std::vector<std::size_t> chain;
        chain.reserve(states.size());
        for (std::size_t index = 0U; index < states.size(); ++index) {
            process_imports(index, sources, states, order, chain, result);
        }
    }

    result.modules.reserve(order.size());
    for (const std::size_t index : order) result.modules.push_back(std::move(states[index].module));
    result.ok = std::none_of(
        result.diagnostics.begin(), result.diagnostics.end(),
        [](const preprocessor_diagnostic& diagnostic) {
            return diagnostic.severity == preprocessor_diagnostic_severity::error;
        });
    return result;
}

} // namespace gore::as::standalone
