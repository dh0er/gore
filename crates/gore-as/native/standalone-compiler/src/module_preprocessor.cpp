#include "gore_as_standalone/module_preprocessor.hpp"

#include <algorithm>
#include <limits>
#include <optional>
#include <regex>
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

enum class source_type_kind { class_type, struct_type, enum_type };

struct source_type_range {
    source_type_kind kind = source_type_kind::class_type;
    std::size_t open = 0U;
    std::size_t close = 0U;
    std::size_t description_index = 0U;
    std::size_t condition_depth = 0U;
    std::size_t declaration_start = 0U;
};

struct source_state {
    lexical_module_description module;
    std::vector<import_description> imports;
    std::vector<source_type_range> type_ranges;
    std::vector<std::vector<std::string>> conditional_lines;
    bool imports_resolved = false;
    bool resolving_imports = false;
};

struct text_replacement {
    std::size_t start = 0U;
    std::size_t end = 0U;
    std::string replacement;
};

struct parsed_specifier {
    std::string name;
    std::string value;
    std::vector<parsed_specifier> list;
};

struct pending_type_macro {
    enum class kind { class_or_struct, enumeration } type = kind::class_or_struct;
    std::size_t start = 0U;
    std::size_t end = 0U;
    std::uint32_t line = 1U;
    std::string arguments;
};

enum class reflection_macro_kind { property, function };

struct reflection_macro {
    reflection_macro_kind kind = reflection_macro_kind::property;
    std::size_t start = 0U;
    std::size_t end = 0U;
    std::size_t name_start = 0U;
    std::size_t name_end = 0U;
    std::uint32_t line = 1U;
    std::string arguments;
    std::string name;
    std::string subject_type;
};

bool starts_at(
    const std::string& value,
    std::size_t position,
    std::string_view prefix) noexcept;
bool is_start_of_identifier(const std::string& code, std::size_t position) noexcept;
void add_diagnostic(
    lexical_preprocess_result& result,
    const preprocessor_source& source,
    std::uint32_t line,
    std::string message);
std::string collect_class_defaults(
    const std::string& code,
    std::size_t open,
    std::size_t close);

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

std::string trim_quotes(std::string value) {
    if (value.size() >= 2U && value.front() == '"' && value.back() == '"') {
        value.erase(value.begin());
        value.pop_back();
    }
    return value;
}

std::vector<parsed_specifier> parse_specifiers(
    const std::string& value,
    std::size_t start,
    std::size_t end);

parsed_specifier parse_specifier(
    const std::string& value,
    const std::size_t start,
    const std::size_t end) {
    parsed_specifier result;
    std::size_t equals = std::string::npos;
    std::size_t bracket_start = std::string::npos;
    std::size_t bracket_depth = 0U;
    bool has_list = false;
    bool in_quotes = false;

    for (std::size_t position = start; position < end; ++position) {
        const char character = value[position];
        switch (character) {
        case '(':
            if (!in_quotes) {
                if (bracket_depth == 0U) bracket_start = position;
                ++bracket_depth;
            }
            break;
        case ')':
            if (!in_quotes && bracket_depth > 0U) {
                --bracket_depth;
                if (bracket_depth == 0U) {
                    has_list = true;
                    result.list = parse_specifiers(
                        value, bracket_start + 1U, position);
                    position = end;
                }
            }
            break;
        case '=':
            if (bracket_depth == 0U && !in_quotes && equals == std::string::npos) {
                result.name = trim_quotes(trim(std::string_view(value).substr(
                    start, position - start)));
                equals = position;
            }
            break;
        case '"':
            in_quotes = !in_quotes;
            break;
        case ' ': case '\t':
            break;
        default:
            if (equals != std::string::npos && bracket_depth == 0U) position = end;
            break;
        }
    }
    if (!has_list) {
        if (equals != std::string::npos) {
            result.value = trim_quotes(trim(std::string_view(value).substr(
                equals + 1U, end - equals - 1U)));
        } else {
            result.name = trim_quotes(trim(std::string_view(value).substr(
                start, end - start)));
        }
    }
    return result;
}

bool validate_reflection_macro_conditions(
    const preprocessor_options& options,
    const preprocessor_source& source,
    const std::vector<std::vector<std::string>>& conditional_lines,
    const std::uint32_t line,
    const std::size_t class_condition_depth,
    lexical_preprocess_result& result) {
    const std::size_t line_index = static_cast<std::size_t>(line - 1U);
    if (line_index >= conditional_lines.size()) return false;
    const std::vector<std::string>& conditions = conditional_lines[line_index];
    for (std::size_t index = class_condition_depth; index < conditions.size(); ++index) {
        const std::string& condition = conditions[index];
        const bool editor_data = condition == "EDITOR" || condition == "EDITORONLY_DATA";
        const auto found = std::find_if(options.flags.begin(), options.flags.end(),
            [&condition](const preprocessor_flag& flag) {
                return flag.name == condition;
            });
        const bool configured_true = found != options.flags.end() && found->value;
        if (!editor_data && !configured_true) {
            add_diagnostic(
                result,
                source,
                line,
                "Cannot put a UPROPERTY or UFUNCTION inside preprocessor conditions other "
                "than EDITOR or flags declared in configuration.");
        }
    }
    return std::any_of(conditions.begin(), conditions.end(), [](const std::string& condition) {
        return condition == "EDITOR" || condition == "EDITORONLY_DATA";
    });
}

std::vector<parsed_specifier> parse_specifiers(
    const std::string& value,
    const std::size_t start,
    const std::size_t end) {
    std::vector<parsed_specifier> result;
    std::size_t bracket_depth = 0U;
    bool in_quotes = false;
    std::size_t term = start;
    for (std::size_t position = start; position < end; ++position) {
        switch (value[position]) {
        case '(':
            if (!in_quotes) ++bracket_depth;
            break;
        case ')':
            if (!in_quotes && bracket_depth > 0U) --bracket_depth;
            break;
        case '"':
            in_quotes = !in_quotes;
            break;
        case ',':
            if (bracket_depth == 0U && !in_quotes) {
                result.push_back(parse_specifier(value, term, position));
                term = position + 1U;
            }
            break;
        default:
            break;
        }
    }
    if (term < end) result.push_back(parse_specifier(value, term, end));
    return result;
}

std::vector<parsed_specifier> parse_specifiers(const std::string& value) {
    return parse_specifiers(value, 0U, value.size());
}

void set_metadata(
    std::vector<preprocessor_metadata>& metadata,
    std::string name,
    std::string value,
    const std::int32_t subject_index = -1) {
    const auto found = std::find_if(metadata.begin(), metadata.end(),
        [&name, subject_index](const preprocessor_metadata& entry) {
            return entry.name == name && entry.subject_index == subject_index;
        });
    if (found != metadata.end()) {
        found->value = std::move(value);
    } else {
        metadata.push_back({std::move(name), std::move(value), subject_index});
    }
}

std::size_t find_scope_close_parenthesis(
    const std::string& value,
    const std::size_t open) noexcept {
    if (open >= value.size() || value[open] != '(') return std::string::npos;
    std::size_t depth = 1U;
    for (std::size_t position = open + 1U; position < value.size(); ++position) {
        if (value[position] == '(') {
            ++depth;
        } else if (value[position] == ')' && --depth == 0U) {
            return position;
        }
    }
    return std::string::npos;
}

std::size_t find_scope_close_brace(
    const std::string& value,
    const std::size_t open) noexcept {
    if (open >= value.size() || value[open] != '{') return std::string::npos;
    std::size_t depth = 1U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    for (std::size_t position = open + 1U; position < value.size(); ++position) {
        const char character = value[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (character == '/' && position + 1U < value.size() &&
            !in_comment && !in_string) {
            if (value[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (value[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < value.size() &&
                   in_block_comment && value[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            bool escaped = false;
            if (in_string) {
                std::size_t check = position;
                while (check > 0U && value[check - 1U] == '\\') {
                    escaped = !escaped;
                    --check;
                }
            }
            if (!escaped) in_string = !in_string;
        } else if (character == '\n') {
            in_line_comment = false;
        } else if (!in_comment && !in_string) {
            if (character == '{') {
                ++depth;
            } else if (character == '}' && --depth == 0U) {
                return position;
            }
        }
    }
    return std::string::npos;
}

std::string joined_namespace(const std::vector<std::string>& stack) {
    std::string result;
    for (const std::string& entry : stack) {
        if (!result.empty()) result += "::";
        result += entry;
    }
    return result;
}

bool token_at(
    const std::string& code,
    const std::size_t position,
    const std::string_view token) noexcept {
    if (!starts_at(code, position, token) || !is_start_of_identifier(code, position)) {
        return false;
    }
    const std::size_t end = position + token.size();
    return end >= code.size() || is_whitespace(code[end]) || code[end] == '(';
}

void apply_class_specifiers(
    const pending_type_macro& macro,
    preprocessed_class_description& description,
    const preprocessor_source& source,
    lexical_preprocess_result& result) {
    for (const parsed_specifier& specifier : parse_specifiers(macro.arguments)) {
        const std::string& name = specifier.name;
        if (name == "NotPlaceable") {
            description.placeable = false;
        } else if (name == "NotBlueprintable") {
            set_metadata(description.metadata, "NotBlueprintable", "true");
            set_metadata(description.metadata, "IsBlueprintBase", "false");
        } else if (name == "Blueprintable") {
            set_metadata(description.metadata, "IsBlueprintBase", "true");
            set_metadata(description.metadata, "Blueprintable", "true");
        } else if (name == "Abstract") {
            description.abstract = true;
        } else if (name == "Transient") {
            description.transient = true;
        } else if (name == "HideDropdown") {
            description.hide_dropdown = true;
        } else if (name == "DefaultToInstanced") {
            description.default_to_instanced = true;
        } else if (name == "EditInlineNew") {
            description.edit_inline_new = true;
        } else if (name == "Deprecated") {
            description.deprecated = true;
        } else if (name == "Config") {
            description.config_name = specifier.value;
        } else if (name == "ClassGroup") {
            set_metadata(description.metadata, "ClassGroupNames", specifier.value);
        } else if (name == "HideCategories" || name == "DefaultConfig" ||
                   name == "ComponentWrapperClass") {
            set_metadata(description.metadata, name, specifier.value);
        } else if (name == "Meta") {
            for (const parsed_specifier& item : specifier.list) {
                if (!item.name.empty()) {
                    set_metadata(description.metadata, item.name, item.value);
                }
            }
        } else {
            add_diagnostic(
                result,
                source,
                macro.line,
                "Unknown class specifier " + name + " on class " +
                    description.class_name + ".");
        }
    }
}

void apply_enum_specifiers(
    const pending_type_macro& macro,
    preprocessed_enum_description& description,
    const preprocessor_source& source,
    lexical_preprocess_result& result,
    const std::int32_t subject_index = -1) {
    for (const parsed_specifier& specifier : parse_specifiers(macro.arguments)) {
        const std::string& name = specifier.name;
        if (subject_index != -1) {
            set_metadata(description.metadata, name, specifier.value, subject_index);
        } else if (name == "Category" || name == "Keywords" ||
                   name == "ToolTip" || name == "DisplayName") {
            set_metadata(description.metadata, name, specifier.value);
        } else if (name == "Meta") {
            for (const parsed_specifier& item : specifier.list) {
                if (!item.name.empty()) {
                    set_metadata(description.metadata, item.name, item.value);
                }
            }
        } else {
            add_diagnostic(
                result,
                source,
                macro.line,
                "Unknown enum specifier " + name + " on enum " +
                    description.enum_name + ".");
        }
    }
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

std::string ascii_fold(std::string value) {
    for (char& character : value) {
        if (character >= 'A' && character <= 'Z') {
            character = static_cast<char>(character - 'A' + 'a');
        }
    }
    return value;
}

std::string make_identifier(const std::string_view value) {
    std::string result;
    result.reserve(value.size());
    for (const char character : value) {
        const bool valid = (character >= 'a' && character <= 'z') ||
            (character >= 'A' && character <= 'Z') ||
            (character >= '0' && character <= '9') || character == '_';
        result.push_back(valid ? character : '_');
    }
    return result;
}

std::string parse_format_expression(const std::string& expression) {
    std::ptrdiff_t equals_position = -1;
    bool found_format = false;

    for (std::ptrdiff_t position = static_cast<std::ptrdiff_t>(expression.size()) - 1;
         position >= 0;
         --position) {
        const char character = expression[static_cast<std::size_t>(position)];
        if (character == ' ' || character == '\t') continue;
        if (character == '=' && !found_format) {
            equals_position = position;
            continue;
        }

        found_format = true;
        if (character == ':') {
            const std::string specifier = expression.substr(
                static_cast<std::size_t>(position + 1));
            if (position > 0 && expression[static_cast<std::size_t>(position - 1)] == '=') {
                const std::string actual = trim(std::string_view(expression).substr(
                    0U, static_cast<std::size_t>(position - 1)));
                return "\"" + actual + " = \"+FString::ApplyFormat((" + actual +
                    "), \"" + specifier + "\")";
            }
            return "FString::ApplyFormat((" +
                expression.substr(0U, static_cast<std::size_t>(position)) +
                "), \"" + specifier + "\")";
        }

        bool valid_format = false;
        switch (character) {
        case '0': case '1': case '2': case '3': case '4':
        case '5': case '6': case '7': case '8': case '9':
        case 'd': case 'x': case 'X': case 'b': case 'c': case 'o':
        case 'n': case 'e': case 'E': case 'f': case 'F': case 'g':
        case 'G': case '%': case ',': case '.': case '-': case '+':
        case '(': case '#':
            valid_format = true;
            break;
        case '<': case '>': case '^': case '=':
            if (position > 0 &&
                expression[static_cast<std::size_t>(position - 1)] != ':') {
                --position;
            }
            valid_format = true;
            break;
        default:
            break;
        }
        if (!valid_format) break;
    }

    if (equals_position != -1) {
        const std::string actual = trim(std::string_view(expression).substr(
            0U, static_cast<std::size_t>(equals_position)));
        return "\"" + actual + " = \"+(" + actual + ")";
    }
    return expression;
}

std::string generate_format_string(const std::string& format) {
    std::string result = "(FString()";
    result.reserve(format.size() + 128U);

    std::size_t start = 0U;
    bool in_expression = false;
    for (std::size_t position = 0U; position < format.size(); ++position) {
        const char character = format[position];
        if (character == '{' && !in_expression) {
            if (position > start) {
                result += ".Append(\"" + format.substr(start, position - start) + "\")";
            }
            if (position + 1U < format.size() && format[position + 1U] == '{') {
                result += ".AppendChar('{')";
                ++position;
                start = position + 1U;
            } else {
                start = position + 1U;
                in_expression = true;
            }
        } else if (character == '}') {
            if (in_expression) {
                if (position > start) {
                    result += ".Append(" +
                        parse_format_expression(format.substr(start, position - start)) + ")";
                }
                start = position + 1U;
                in_expression = false;
            } else if (position + 1U < format.size() && format[position + 1U] == '}') {
                if (position > start) {
                    result += ".Append(\"" + format.substr(start, position - start) + "\")";
                }
                result += ".AppendChar('}')";
                ++position;
                start = position + 1U;
            }
        }
    }
    if (!in_expression && format.size() > start) {
        result += ".Append(\"" + format.substr(start) + "\")";
    }
    result.push_back(')');
    return result;
}

std::string apply_replacements(
    const std::string& code,
    std::vector<text_replacement> replacements) {
    if (replacements.empty()) return code;
    std::sort(replacements.begin(), replacements.end(),
        [](const text_replacement& left, const text_replacement& right) {
            return left.start < right.start;
        });

    std::string result;
    result.reserve(code.size());
    std::size_t previous = 0U;
    for (const text_replacement& replacement : replacements) {
        if (replacement.start < previous || replacement.end < replacement.start ||
            replacement.end > code.size()) {
            continue;
        }
        result.append(code, previous, replacement.start - previous);
        result += replacement.replacement;
        previous = replacement.end;
    }
    result.append(code, previous, code.size() - previous);
    return result;
}

void adjust_type_ranges_for_replacements(
    std::vector<source_type_range>& type_ranges,
    const std::vector<text_replacement>& replacements) {
    for (source_type_range& range : type_ranges) {
        std::ptrdiff_t open_delta = 0;
        std::ptrdiff_t close_delta = 0;
        for (const text_replacement& replacement : replacements) {
            const std::ptrdiff_t delta =
                static_cast<std::ptrdiff_t>(replacement.replacement.size()) -
                static_cast<std::ptrdiff_t>(replacement.end - replacement.start);
            if (replacement.end <= range.open) open_delta += delta;
            if (replacement.end <= range.close) close_delta += delta;
        }
        range.open = static_cast<std::size_t>(
            static_cast<std::ptrdiff_t>(range.open) + open_delta);
        range.close = static_cast<std::size_t>(
            static_cast<std::ptrdiff_t>(range.close) + close_delta);
        std::ptrdiff_t declaration_delta = 0;
        for (const text_replacement& replacement : replacements) {
            const std::ptrdiff_t delta =
                static_cast<std::ptrdiff_t>(replacement.replacement.size()) -
                static_cast<std::ptrdiff_t>(replacement.end - replacement.start);
            if (replacement.end <= range.declaration_start) declaration_delta += delta;
        }
        range.declaration_start = static_cast<std::size_t>(
            static_cast<std::ptrdiff_t>(range.declaration_start) + declaration_delta);
    }
}

std::string generate_static_name(
    const std::string& name,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices,
    bool& limit_exceeded) {
    const std::string key = ascii_fold(name);
    const auto found = static_name_indices.find(key);
    if (found != static_name_indices.end()) {
        return "__STATIC_NAME(" + std::to_string(found->second) + ")";
    }
    if (static_names.size() >= max_preprocessor_static_names) {
        limit_exceeded = true;
        return "__STATIC_NAME(0)";
    }
    const std::size_t index = static_names.size();
    static_names.push_back(name);
    static_name_indices.emplace(key, index);
    return "__STATIC_NAME(" + std::to_string(index) + ")";
}

void lower_name_and_format_literals(
    std::string& code,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices,
    bool& static_name_limit_exceeded,
    std::vector<source_type_range>* type_ranges) {
    std::vector<text_replacement> replacements;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::size_t name_literal_start = std::string::npos;
    std::size_t format_string_start = std::string::npos;

    for (std::size_t position = 0U; position < code.size(); ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (character == 'n' && position + 1U < code.size() &&
            code[position + 1U] == '"' && !in_comment && !in_string) {
            name_literal_start = position;
        } else if (character == 'f' && position + 1U < code.size() &&
                   code[position + 1U] == '"' && !in_comment && !in_string) {
            format_string_start = position;
        }

        if (character == '/' && position + 1U < code.size() &&
            !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
                continue;
            }
            if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
                continue;
            }
        } else if (character == '*' && position + 1U < code.size() &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
            continue;
        } else if (character == '"' && !in_comment) {
            bool escaped = false;
            if (in_string) {
                std::size_t check = position;
                while (check > 0U && code[check - 1U] == '\\') {
                    escaped = !escaped;
                    --check;
                }
            }
            if (!escaped) {
                in_string = !in_string;
                if (!in_string && name_literal_start != std::string::npos) {
                    replacements.push_back({
                        name_literal_start,
                        position + 1U,
                        generate_static_name(
                            code.substr(name_literal_start + 2U,
                                position - name_literal_start - 2U),
                            static_names,
                            static_name_indices,
                            static_name_limit_exceeded)});
                    name_literal_start = std::string::npos;
                } else if (!in_string && format_string_start != std::string::npos) {
                    replacements.push_back({
                        format_string_start,
                        position + 1U,
                        generate_format_string(code.substr(
                            format_string_start + 2U,
                            position - format_string_start - 2U))});
                    format_string_start = std::string::npos;
                }
            }
        } else if (character == '\n') {
            in_line_comment = false;
            name_literal_start = std::string::npos;
            format_string_start = std::string::npos;
        }
    }
    std::sort(replacements.begin(), replacements.end(),
        [](const text_replacement& left, const text_replacement& right) {
            return left.start < right.start;
        });
    if (type_ranges != nullptr) {
        adjust_type_ranges_for_replacements(*type_ranges, replacements);
    }
    code = apply_replacements(code, std::move(replacements));
}

void lower_range_based_for(std::string& code) {
    static const std::regex pattern(
        R"(for(\s*)\(([^:;{]*):([^:;{\n][^;{\n]*)\)(\s*)(\{|.*;))",
        std::regex::ECMAScript | std::regex::optimize);

    std::string result;
    std::size_t previous = 0U;
    bool matched = false;
    for (std::sregex_iterator iterator(code.begin(), code.end(), pattern), end;
         iterator != end;
         ++iterator) {
        matched = true;
        const std::smatch& match = *iterator;
        const std::size_t start = static_cast<std::size_t>(match.position());
        result.append(code, previous, start - previous);

        const std::string final_group = match[5].str();
        const bool single_line = final_group != "{";
        std::string store_type = trim(match[2].str());
        std::ptrdiff_t start_of_name = static_cast<std::ptrdiff_t>(store_type.size()) - 1;
        while (start_of_name >= 0) {
            const char character = store_type[static_cast<std::size_t>(start_of_name)];
            if ((character >= 'a' && character <= 'z') ||
                (character >= 'A' && character <= 'Z') ||
                (character >= '0' && character <= '9') || character == '_') {
                --start_of_name;
            } else {
                break;
            }
        }
        const std::size_t split = start_of_name < 0
            ? 0U
            : static_cast<std::size_t>(start_of_name);
        store_type.insert(split, " __auto_constref_type");

        std::string suffix;
        if (!single_line) {
            suffix = match[4].str();
        } else if (final_group == ";") {
            suffix = ";";
        }
        result += "for" + match[1].str() + "(auto _Iterator = " + match[3].str() +
            ".Iterator();_Iterator.CanProceed; )" + suffix + "{ " + store_type +
            " = _Iterator.Proceed();";
        if (single_line) {
            result += match[4].str();
            result += final_group;
            result += "}";
        }
        previous = start + static_cast<std::size_t>(match.length());
    }
    if (!matched) return;
    result.append(code, previous, code.size() - previous);
    code = std::move(result);
}

void lower_literal_assets(
    std::string& code,
    std::vector<std::string>& post_init_functions) {
    static const std::regex pattern(
        R"(asset\s+([A-Za-z0-9_]+)\s+of\s+([A-Za-z0-9]+)\s*($|\r|\n))",
        std::regex::ECMAScript | std::regex::optimize);
    std::string result;
    std::size_t previous = 0U;
    bool matched = false;
    for (std::sregex_iterator iterator(code.begin(), code.end(), pattern), end;
         iterator != end;
         ++iterator) {
        matched = true;
        const std::smatch& match = *iterator;
        const std::size_t start = static_cast<std::size_t>(match.position());
        result.append(code, previous, start - previous);
        const std::string name = match[1].str();
        const std::string type = match[2].str();
        result += type + " __Asset_" + name + ";" +
            type + " Get" + name + "() property" +
            "{\tif (__Asset_" + name + " != nullptr)\t\treturn __Asset_" + name + ";" +
            "\t__Asset_" + name + " = Cast<" + type + ">(__CreateLiteralAsset(" + type +
            ", \"" + name + "\"));" +
            "\tif (__Asset_" + name + " == nullptr) return nullptr;" +
            "\t__Init_" + name + "(__Asset_" + name + ");" +
            "\t__PostLiteralAssetSetup(__Asset_" + name + ", \"" + name + "\");" +
            "\treturn __Asset_" + name + ";} " +
            "void __Init_" + name + "(" + type + " " + name +
            ") external_implicit_this\n";
        post_init_functions.push_back("Get" + name);
        previous = start + static_cast<std::size_t>(match.length());
    }
    if (!matched) return;
    result.append(code, previous, code.size() - previous);
    code = std::move(result);
}

std::string filename_to_module_name(const std::string& filename) {
    return replace_all(replace_all(filename, ".as", ""), "/", ".");
}

std::string effective_module_name(const preprocessor_source& source) {
    if (!source.module_name.empty()) return source.module_name;
    return filename_to_module_name(source.relative_path);
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
    const std::vector<preprocessor_base_module>& base_modules,
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
    if (options.static_names.size() > max_preprocessor_static_names) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "static-name seed count exceeds the bounded maximum"});
        return false;
    }
    if (options.blueprint_event_argument_specializations.size() >
        max_preprocessor_flags) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "blueprint-event argument specialization count exceeds the bounded maximum"});
        return false;
    }
    if (options.native_super_types.size() > max_preprocessor_static_names) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "native-super type count exceeds the bounded maximum"});
        return false;
    }
    if (base_modules.size() > max_preprocessor_base_modules) {
        result.diagnostics.push_back({
            preprocessor_diagnostic_severity::error, {}, 1U, 1U,
            "base module count exceeds the bounded maximum"});
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

    std::unordered_set<std::string> static_name_keys;
    static_name_keys.reserve(options.static_names.size());
    for (const std::string& name : options.static_names) {
        if (!valid_text_field(name, true) ||
            !static_name_keys.insert(ascii_fold(name)).second) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "static-name seed contains an invalid or duplicate name"});
            return false;
        }
    }

    std::unordered_set<std::string> specialization_names;
    specialization_names.reserve(options.blueprint_event_argument_specializations.size());
    for (const std::string& name : options.blueprint_event_argument_specializations) {
        if (!valid_text_field(name, false) ||
            !specialization_names.insert(name).second) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "blueprint-event argument specializations contain an invalid or duplicate name"});
            return false;
        }
    }

    std::string previous_native_super;
    std::unordered_set<std::string> native_super_paths;
    for (const native_super_type& type : options.native_super_types) {
        if (!valid_text_field(type.angelscript_type_name, false) ||
            !valid_text_field(type.unreal_class_path, false) ||
            type.property_offset >
                static_cast<std::uint64_t>((std::numeric_limits<std::int32_t>::max)()) ||
            !native_super_paths.insert(type.unreal_class_path).second ||
            (!previous_native_super.empty() &&
             previous_native_super >= type.angelscript_type_name)) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "native-super types are invalid, duplicated or out of canonical order"});
            return false;
        }
        previous_native_super = type.angelscript_type_name;
    }

    std::unordered_set<std::string> base_module_names;
    std::unordered_set<std::string> base_class_names;
    std::size_t base_class_count = 0U;
    for (const preprocessor_base_module& module : base_modules) {
        if (!valid_text_field(module.module_name, false) ||
            !base_module_names.insert(module.module_name).second) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "base modules contain an invalid or duplicate module name"});
            return false;
        }
        if (module.classes.size() > max_preprocessor_base_classes ||
            base_class_count > max_preprocessor_base_classes - module.classes.size()) {
            result.diagnostics.push_back({
                preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                "base class count exceeds the bounded maximum"});
            return false;
        }
        base_class_count += module.classes.size();
        for (const preprocessor_base_class& type : module.classes) {
            if (!valid_text_field(type.class_name, false) ||
                !valid_text_field(type.name_space, true) ||
                !valid_text_field(type.super_class, true) ||
                !valid_text_field(type.code_super_class, type.is_struct) ||
                !base_class_names.insert(type.class_name).second) {
                result.diagnostics.push_back({
                    preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                    "base classes contain invalid text or a duplicate class name"});
                return false;
            }
        }
    }

    std::unordered_set<std::string> source_module_names;
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
        const std::string derived_module_name = filename_to_module_name(source.relative_path);
        const std::string module_name = effective_module_name(source);
        if (module_name.empty()) {
            add_diagnostic(result, source, 1U, "source path produces an empty module name");
            return false;
        }
        if (!source.module_name.empty() && source.module_name != derived_module_name) {
            add_diagnostic(
                result, source, 1U,
                "explicit source module name does not match its relative path");
            return false;
        }
        if (!source_module_names.insert(module_name).second) {
            add_diagnostic(result, source, 1U, "overlay contains a duplicate module name");
            return false;
        }
        const bool exists_in_base = base_module_names.find(module_name) != base_module_names.end();
        if (source.overlay_operation == preprocessor_source::operation::add && exists_in_base) {
            add_diagnostic(result, source, 1U, "add overlay collides with a base module");
            return false;
        }
        if (source.overlay_operation == preprocessor_source::operation::edit &&
            !exists_in_base) {
            add_diagnostic(result, source, 1U, "edit overlay does not name a base module");
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

std::optional<pending_type_macro> parse_type_macro(
    const std::string& code,
    const std::size_t position,
    const std::string_view prefix,
    const pending_type_macro::kind kind,
    const std::uint32_t line) {
    if (!starts_at(code, position, prefix)) return std::nullopt;
    const std::size_t open = position + prefix.size() - 1U;
    const std::size_t close = find_scope_close_parenthesis(code, open);
    if (close == std::string::npos) return std::nullopt;
    return pending_type_macro{
        kind,
        position,
        close + 1U,
        line,
        code.substr(open + 1U, close - open - 1U)};
}

void strip_comments_from_line(std::string& line) {
    bool in_line_comment = false;
    bool in_string = false;
    bool in_block_comment = false;
    for (std::size_t index = 0U; index < line.size(); ++index) {
        const char character = line[index];
        if (character == '"' && !in_line_comment && !in_block_comment) {
            in_string = !in_string;
        }
        if (character == '/' && index + 1U < line.size() && !in_string &&
            !in_line_comment && !in_block_comment) {
            if (line[index + 1U] == '/') {
                in_line_comment = true;
            } else if (line[index + 1U] == '*') {
                in_block_comment = true;
            }
        }
        if (in_block_comment || in_line_comment) line[index] = ' ';
        if (character == '*' && index + 1U < line.size() && in_block_comment &&
            line[index + 1U] == '/') {
            line[index + 1U] = ' ';
            in_block_comment = false;
        }
    }
}

std::optional<reflection_macro> parse_reflection_macro(
    const std::string& code,
    const std::size_t position,
    const std::size_t range_end,
    const reflection_macro_kind kind,
    const std::uint32_t line) {
    constexpr std::string_view property_prefix = "UPROPERTY(";
    constexpr std::string_view function_prefix = "UFUNCTION(";
    const std::string_view prefix = kind == reflection_macro_kind::property
        ? property_prefix
        : function_prefix;
    if (!starts_at(code, position, prefix)) return std::nullopt;
    const std::size_t open = position + prefix.size() - 1U;
    const std::size_t close = find_scope_close_parenthesis(code, open);
    if (close == std::string::npos || close >= range_end) return std::nullopt;

    std::size_t subject_end = close + 1U;
    std::size_t parenthesis_depth = 0U;
    bool in_string = false;
    bool in_line_comment = false;
    bool in_block_comment = false;
    for (; subject_end < range_end; ++subject_end) {
        const char character = code[subject_end];
        const bool in_comment = in_line_comment || in_block_comment;
        if (character == '/' && subject_end + 1U < range_end &&
            !in_comment && !in_string) {
            if (code[subject_end + 1U] == '/') {
                in_line_comment = true;
                ++subject_end;
            } else if (code[subject_end + 1U] == '*') {
                in_block_comment = true;
                ++subject_end;
            }
        } else if (character == '*' && subject_end + 1U < range_end &&
                   in_block_comment && code[subject_end + 1U] == '/') {
            in_block_comment = false;
            ++subject_end;
        } else if (character == '"' && !in_comment) {
            in_string = !in_string;
        } else if (character == '\n') {
            in_line_comment = false;
        } else if (!in_comment && !in_string) {
            if (kind == reflection_macro_kind::function && character == '(' &&
                parenthesis_depth == 0U) {
                break;
            }
            if (kind == reflection_macro_kind::property &&
                parenthesis_depth == 0U && (character == ';' || character == '=')) {
                break;
            }
            if (character == '(') {
                ++parenthesis_depth;
            } else if (character == ')' && parenthesis_depth > 0U) {
                --parenthesis_depth;
            }
        }
    }
    if (subject_end >= range_end) return std::nullopt;

    std::size_t end_of_word = subject_end;
    while (end_of_word > 0U &&
           (code[end_of_word - 1U] == ' ' || code[end_of_word - 1U] == '\t')) {
        --end_of_word;
    }
    std::size_t start_of_word = end_of_word;
    while (start_of_word > 0U && code[start_of_word - 1U] != ' ' &&
           code[start_of_word - 1U] != '\t') {
        --start_of_word;
    }
    if (start_of_word == end_of_word) return std::nullopt;

    reflection_macro macro;
    macro.kind = kind;
    macro.start = position;
    macro.end = close + 1U;
    macro.name_start = start_of_word;
    macro.name_end = end_of_word;
    macro.line = line;
    macro.arguments = code.substr(open + 1U, close - open - 1U);
    macro.name = code.substr(start_of_word, end_of_word - start_of_word);
    if (kind == reflection_macro_kind::property) {
        std::size_t end_of_type = start_of_word;
        while (end_of_type > 0U && is_whitespace(code[end_of_type - 1U])) --end_of_type;
        std::size_t start_of_type = end_of_type;
        while (start_of_type > 0U && code[start_of_type - 1U] != '\n' &&
               code[start_of_type - 1U] != ')') {
            --start_of_type;
        }
        macro.subject_type = trim(std::string_view(code).substr(
            start_of_type, end_of_type - start_of_type));
    }
    return macro;
}

void apply_default_property_access(
    preprocessed_property_description& property,
    const property_edit_specifier edit,
    const property_blueprint_specifier blueprint) {
    switch (edit) {
    case property_edit_specifier::edit_anywhere:
        property.editable_on_defaults = true;
        property.editable_on_instance = true;
        break;
    case property_edit_specifier::edit_instance_only:
        property.editable_on_instance = true;
        break;
    case property_edit_specifier::edit_defaults_only:
        property.editable_on_defaults = true;
        break;
    case property_edit_specifier::not_editable:
        break;
    }
    switch (blueprint) {
    case property_blueprint_specifier::blueprint_read_write:
        property.blueprint_readable = true;
        property.blueprint_writable = true;
        break;
    case property_blueprint_specifier::blueprint_read_only:
        property.blueprint_readable = true;
        break;
    case property_blueprint_specifier::blueprint_hidden:
        break;
    }
}

std::optional<std::int32_t> replication_condition_value(const std::string& value) {
    static const std::pair<std::string_view, std::int32_t> values[] = {
        {"None", 0}, {"InitialOnly", 1}, {"OwnerOnly", 2}, {"SkipOwner", 3},
        {"SimulatedOnly", 4}, {"AutonomousOnly", 5}, {"SimulatedOrPhysics", 6},
        {"InitialOrOwner", 7}, {"Custom", 8}, {"ReplayOrOwner", 9},
        {"ReplayOnly", 10}, {"SimulatedOnlyNoReplay", 11},
        {"SimulatedOrPhysicsNoReplay", 12}, {"SkipReplay", 13},
    };
    for (const auto& [name, number] : values) {
        if (value == name) return number;
    }
    return std::nullopt;
}

struct parsed_function_text {
    std::string arguments;
    std::vector<std::string> argument_names;
    std::vector<std::string> argument_types;
    std::string return_type;
    std::string access_specifier;
    bool const_method = false;
    bool property_method = false;
    bool const_return = false;
};

parsed_function_text parse_function_text(
    const std::string& code,
    const reflection_macro& macro) {
    parsed_function_text result;
    const std::size_t open = code.find('(', macro.name_end);
    if (open == std::string::npos) return result;
    const std::size_t close = find_scope_close_parenthesis(code, open);
    if (close == std::string::npos) return result;
    result.arguments = code.substr(open + 1U, close - open - 1U);
    for (char& character : result.arguments) {
        if (character == '\n' || character == '\r') character = ' ';
    }

    std::size_t term = open + 1U;
    std::size_t angle_depth = 0U;
    std::size_t parenthesis_depth = 0U;
    const auto record_argument = [&](const std::size_t end) {
        std::string argument = trim(std::string_view(code).substr(term, end - term));
        std::size_t equals = std::string::npos;
        std::size_t local_angle = 0U;
        std::size_t local_parenthesis = 0U;
        for (std::size_t index = 0U; index < argument.size(); ++index) {
            if (argument[index] == '<') ++local_angle;
            else if (argument[index] == '>' && local_angle > 0U) --local_angle;
            else if (argument[index] == '(') ++local_parenthesis;
            else if (argument[index] == ')' && local_parenthesis > 0U) --local_parenthesis;
            else if (argument[index] == '=' && local_angle == 0U && local_parenthesis == 0U) {
                equals = index;
                break;
            }
        }
        if (equals != std::string::npos) argument = trim(
            std::string_view(argument).substr(0U, equals));
        if (argument.empty()) return;
        std::size_t end_of_name = argument.size();
        while (end_of_name > 0U && is_whitespace(argument[end_of_name - 1U])) --end_of_name;
        std::size_t start_of_name = end_of_name;
        while (start_of_name > 0U && !is_whitespace(argument[start_of_name - 1U])) {
            --start_of_name;
        }
        if (start_of_name == end_of_name) return;
        result.argument_names.push_back(
            argument.substr(start_of_name, end_of_name - start_of_name));
        result.argument_types.push_back(trim(
            std::string_view(argument).substr(0U, start_of_name)));
    };
    for (std::size_t position = open + 1U; position < close; ++position) {
        const char character = code[position];
        if (character == '<') ++angle_depth;
        else if (character == '>' && angle_depth > 0U) --angle_depth;
        else if (character == '(') ++parenthesis_depth;
        else if (character == ')' && parenthesis_depth > 0U) --parenthesis_depth;
        else if (character == ',' && angle_depth == 0U && parenthesis_depth == 0U) {
            record_argument(position);
            term = position + 1U;
        }
    }
    if (term < close) record_argument(close);

    std::size_t suffix = close + 1U;
    while (suffix < code.size()) {
        while (suffix < code.size() &&
               (code[suffix] == ' ' || code[suffix] == '\t')) ++suffix;
        if (starts_at(code, suffix, "const")) {
            result.const_method = true;
            suffix += 5U;
        } else if (starts_at(code, suffix, "property")) {
            result.property_method = true;
            suffix += 8U;
        } else {
            break;
        }
    }

    std::size_t end_of_type = macro.name_start;
    while (end_of_type > 0U && is_whitespace(code[end_of_type - 1U])) --end_of_type;
    std::size_t start_of_type = end_of_type;
    while (start_of_type > 0U && !is_whitespace(code[start_of_type - 1U])) {
        --start_of_type;
    }
    result.return_type = trim(std::string_view(code).substr(
        start_of_type, end_of_type - start_of_type));

    std::size_t qualifier_end = start_of_type;
    while (qualifier_end > 0U) {
        while (qualifier_end > 0U && is_whitespace(code[qualifier_end - 1U])) {
            --qualifier_end;
        }
        std::size_t qualifier_start = qualifier_end;
        while (qualifier_start > 0U && !is_whitespace(code[qualifier_start - 1U])) {
            --qualifier_start;
        }
        const std::string qualifier = code.substr(
            qualifier_start, qualifier_end - qualifier_start);
        if (qualifier == "const") {
            result.const_return = true;
            qualifier_end = qualifier_start;
        } else if (qualifier == "private" || qualifier == "protected") {
            result.access_specifier = qualifier;
            break;
        } else {
            break;
        }
    }
    return result;
}

std::string push_argument_suffix(
    std::string type,
    const std::unordered_set<std::string>& specializations) {
    if (type.find('<') != std::string::npos) return {};
    if (starts_at(type, 0U, "const ")) type.erase(0U, 6U);
    const std::size_t reference = type.rfind('&');
    if (reference != std::string::npos) type.erase(reference);
    type = trim(type);
    return specializations.find(type) == specializations.end()
        ? std::string{}
        : "__" + type;
}

std::string return_initializer(
    const std::string& type,
    const bool script_float_is_float64) {
    if (type == "bool") return " = false";
    if (type == "int" || type == "int16" || type == "int32" || type == "int64" ||
        type == "int8" || type == "uint" || type == "uint16" || type == "uint32" ||
        type == "uint64") return " = 0";
    if (type == "float32") return " = 0.f";
    if (type == "float64" || type == "double") return " = 0.0";
    if (type == "float") return script_float_is_float64 ? " = 0.0" : " = 0.f";
    return {};
}

std::string generate_blueprint_event_wrapper(
    const preprocessor_options& options,
    const std::string& code,
    const reflection_macro& macro,
    const std::string& function_name,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices,
    bool& static_name_limit_exceeded) {
    const parsed_function_text function = parse_function_text(code, macro);
    std::unordered_set<std::string> specializations(
        options.blueprint_event_argument_specializations.begin(),
        options.blueprint_event_argument_specializations.end());
    std::string qualified_return = function.return_type;
    if (function.const_return) qualified_return = "const " + qualified_return;
    if (!function.access_specifier.empty()) {
        qualified_return = function.access_specifier + " " + qualified_return;
    }

    std::string wrapper = qualified_return + " " + function_name + "(" +
        function.arguments + ") " + (function.const_method ? "const " : "") +
        "final" + (function.property_method ? " property" : "") + " {";
    for (std::size_t index = 0U;
         index < function.argument_names.size() && index < function.argument_types.size();
         ++index) {
        const std::string& argument = function.argument_names[index];
        const std::string& type = function.argument_types[index];
        const bool reference = type.find('&') != std::string::npos;
        const bool const_reference = type.find("const ") != std::string::npos;
        if (reference && !const_reference) {
            wrapper += " __Evt_PushArgumentRef" + push_argument_suffix(type, specializations) +
                "(" + argument + ");";
        } else {
            wrapper += " __Evt_PushArgument" + push_argument_suffix(type, specializations) +
                "(" + argument + ");";
        }
    }
    if (function.return_type != "void") {
        wrapper += " " + function.return_type + " __ReturnValue" +
            return_initializer(function.return_type, options.script_float_is_float64) +
            "; __Evt_PushArgumentRef" +
            push_argument_suffix(function.return_type, specializations) +
            "(__ReturnValue);";
    }
    wrapper += " __Evt_Execute(this, " + generate_static_name(
        function_name,
        static_names,
        static_name_indices,
        static_name_limit_exceeded) + ");";
    if (function.return_type != "void") wrapper += " return __ReturnValue;";
    wrapper += "}";
    return wrapper;
}

void process_property_macro(
    const preprocessor_options& options,
    const preprocessor_source& source,
    std::string& code,
    const reflection_macro& macro,
    preprocessed_class_description& description,
    lexical_preprocess_result& result,
    const bool editor_only) {
    preprocessed_property_description property;
    property.property_name = macro.name;
    property.literal_type = macro.subject_type;
    property.line = macro.line;
    apply_default_property_access(
        property,
        description.is_struct ? options.default_struct_property_edit
                              : options.default_property_edit,
        options.default_property_blueprint);
    if (editor_only) set_metadata(property.metadata, "EditorOnly", "");

    bool had_show_on_actor = false;
    bool had_root_component = false;
    bool had_attachment = false;
    bool is_default_component = false;
    bool is_override_component = false;
    for (const parsed_specifier& specifier : parse_specifiers(macro.arguments)) {
        const std::string& name = specifier.name;
        if (name == "BlueprintReadWrite") {
            property.blueprint_readable = true;
            property.blueprint_writable = true;
        } else if (name == "BlueprintReadOnly") {
            property.blueprint_readable = true;
            property.blueprint_writable = false;
        } else if (name == "BlueprintHidden") {
            property.blueprint_readable = false;
            property.blueprint_writable = false;
        } else if (name == "EditInstanceOnly") {
            property.editable_on_defaults = false;
            property.editable_on_instance = true;
        } else if (name == "EditDefaultsOnly") {
            property.editable_on_defaults = true;
            property.editable_on_instance = false;
        } else if (name == "EditAnywhere") {
            property.editable_on_defaults = true;
            property.editable_on_instance = true;
        } else if (name == "NotVisible" || name == "NotEditable") {
            property.editable_on_defaults = false;
            property.editable_on_instance = false;
        } else if (name == "EditConst") {
            property.edit_const = true;
        } else if (name == "VisibleAnywhere") {
            property.edit_const = true;
            property.editable_on_defaults = true;
            property.editable_on_instance = true;
        } else if (name == "VisibleInstanceOnly") {
            property.edit_const = true;
            property.editable_on_defaults = false;
            property.editable_on_instance = true;
        } else if (name == "VisibleDefaultsOnly") {
            property.edit_const = true;
            property.editable_on_defaults = true;
            property.editable_on_instance = false;
        } else if (name == "BindWidget") {
            set_metadata(property.metadata, "BindWidget", "");
            property.editable_on_defaults = false;
            property.editable_on_instance = false;
            property.blueprint_writable = false;
            property.blueprint_readable = true;
        } else if (!options.angelscript_haze && name == "Replicated") {
            property.replicated = true;
        } else if (!options.angelscript_haze && name == "ReplicationCondition") {
            const auto condition = replication_condition_value(specifier.value);
            if (condition.has_value()) {
                property.replication_condition = *condition;
            } else {
                add_diagnostic(
                    result, source, macro.line,
                    "Unknown ReplicationCondition " + specifier.value + " on property " +
                        description.class_name + "::" + property.property_name + ".");
            }
        } else if (!options.angelscript_haze && name == "ReplicatedUsing") {
            if (!specifier.value.empty()) {
                property.replicated = true;
                property.rep_notify = true;
                set_metadata(property.metadata, "ReplicatedUsing", specifier.value);
            } else {
                add_diagnostic(
                    result, source, macro.line,
                    "No function specified for ReplicatedUsing on property " +
                        description.class_name + "::" + property.property_name + ".");
            }
        } else if (!options.angelscript_haze && name == "NotReplicated") {
            if (!description.is_struct) {
                add_diagnostic(
                    result, source, macro.line,
                    "The NotReplicated specifier is only allowed structs.");
            } else {
                property.skip_replication = true;
            }
        } else if (name == "SkipSerialization") {
            property.skip_serialization = true;
        } else if (name == "SaveGame") {
            property.save_game = true;
        } else if (name == "AdvancedDisplay") {
            property.advanced_display = true;
        } else if (name == "Transient") {
            property.transient = true;
        } else if (name == "Config") {
            property.config = true;
        } else if (name == "Interp") {
            property.interp = true;
        } else if (name == "AssetRegistrySearchable") {
            property.asset_registry_searchable = true;
        } else if (name == "NoClear") {
            property.no_clear = true;
        } else if (name == "OverrideComponent") {
            property.edit_const = false;
            property.editable_on_defaults = false;
            property.editable_on_instance = false;
            property.blueprint_writable = false;
            property.blueprint_readable = false;
            property.instanced_reference = true;
            set_metadata(property.metadata, name, specifier.value);
            is_override_component = true;
        } else if (name == "DefaultComponent") {
            if (!had_show_on_actor) {
                property.edit_const = false;
                property.editable_on_defaults = true;
                property.editable_on_instance = false;
            }
            property.blueprint_writable = false;
            property.blueprint_readable = true;
            property.instanced_reference = true;
            is_default_component = true;
            set_metadata(property.metadata, "EditInlineDefaults", "true");
            set_metadata(property.metadata, name, "True");
        } else if (name == "ShowOnActor") {
            had_show_on_actor = true;
            property.edit_const = false;
            property.editable_on_defaults = true;
            property.editable_on_instance = true;
            set_metadata(property.metadata, "EditInline", "true");
        } else if (name == "Category" || name == "Keywords" || name == "ToolTip" ||
                   name == "DisplayName" || name == "EditInline" ||
                   name == "ExposeOnSpawn" || name == "EditFixedSize" ||
                   name == "BlueprintProtected") {
            set_metadata(property.metadata, name, specifier.value);
        } else if (name == "RootComponent") {
            had_root_component = true;
            set_metadata(property.metadata, name, specifier.value);
        } else if (name == "Attach" || name == "AttachSocket") {
            had_attachment = true;
            set_metadata(property.metadata, name, specifier.value);
        } else if (name == "Meta") {
            for (const parsed_specifier& item : specifier.list) {
                if (!item.name.empty()) set_metadata(property.metadata, item.name, item.value);
            }
        } else if (name == "Instanced") {
            property.persistent_instance = true;
        } else if (name == "BlueprintSetter" || name == "BlueprintGetter") {
            if (!specifier.value.empty()) {
                set_metadata(property.metadata, name, specifier.value);
            } else {
                add_diagnostic(
                    result, source, macro.line,
                    "No function specified for " + name + " on property " +
                        description.class_name + "::" + property.property_name + ".");
            }
        } else {
            add_diagnostic(
                result, source, macro.line,
                "Unknown property specifier " + name + " on property " +
                    description.class_name + "::" + property.property_name + ".");
        }
    }

    replace_with_blank(code, macro.start, macro.end);
    if (had_show_on_actor && !property.instanced_reference && !property.persistent_instance) {
        add_diagnostic(
            result, source, macro.line,
            "ShowOnActor can only be used on default components in actors");
    }
    if (!is_default_component) {
        if (had_attachment) {
            add_diagnostic(
                result, source, macro.line,
                "Attachments can only be specified on DefaultComponents");
        }
        if (had_root_component) {
            add_diagnostic(
                result, source, macro.line,
                "RootComponent can only be specified on DefaultComponents");
        }
    } else if (is_override_component) {
        add_diagnostic(
            result, source, macro.line,
            "OverrideComponent and DefaultComponent should not be used simultaneously");
    }
    description.properties.push_back(std::move(property));
}

std::uint32_t line_at(const std::string& code, const std::size_t position) noexcept {
    std::uint32_t line = 1U;
    for (std::size_t index = 0U; index < position && index < code.size(); ++index) {
        if (code[index] == '\n' && line != (std::numeric_limits<std::uint32_t>::max)()) {
            ++line;
        }
    }
    return line;
}

void process_property_macros(
    const preprocessor_options& options,
    const preprocessor_source& source,
    source_state& state,
    lexical_preprocess_result& result) {
    std::string& code = state.module.code.front().conditioned_code;
    for (const source_type_range& range : state.type_ranges) {
        if (range.kind == source_type_kind::enum_type ||
            range.description_index >= state.module.classes.size() ||
            range.open >= range.close || range.close > code.size()) {
            continue;
        }
        preprocessed_class_description& description =
            state.module.classes[range.description_index];
        std::size_t depth = 1U;
        bool in_line_comment = false;
        bool in_block_comment = false;
        bool in_string = false;
        std::uint32_t line = line_at(code, range.open);
        for (std::size_t position = range.open + 1U; position < range.close; ++position) {
            const char character = code[position];
            const bool in_comment = in_line_comment || in_block_comment;
            if (!in_comment && !in_string && depth == 1U && character == 'U' &&
                token_at(code, position, "UPROPERTY")) {
                const auto macro = parse_reflection_macro(
                    code,
                    position,
                    range.close,
                    reflection_macro_kind::property,
                    line);
                if (macro.has_value()) {
                    const bool editor_only = validate_reflection_macro_conditions(
                        options,
                        source,
                        state.conditional_lines,
                        macro->line,
                        range.condition_depth,
                        result);
                    process_property_macro(
                        options,
                        source,
                        code,
                        *macro,
                        description,
                        result,
                        editor_only);
                    position = macro->end - 1U;
                    continue;
                }
            }
            if (character == '/' && position + 1U < range.close &&
                !in_comment && !in_string) {
                if (code[position + 1U] == '/') {
                    in_line_comment = true;
                    ++position;
                } else if (code[position + 1U] == '*') {
                    in_block_comment = true;
                    ++position;
                }
            } else if (character == '*' && position + 1U < range.close &&
                       in_block_comment && code[position + 1U] == '/') {
                in_block_comment = false;
                ++position;
            } else if (character == '"' && !in_comment) {
                bool escaped = false;
                if (in_string) {
                    std::size_t check = position;
                    while (check > 0U && code[check - 1U] == '\\') {
                        escaped = !escaped;
                        --check;
                    }
                }
                if (!escaped) in_string = !in_string;
            } else if (!in_comment && !in_string && character == '{') {
                ++depth;
            } else if (!in_comment && !in_string && character == '}' && depth > 0U) {
                --depth;
            } else if (character == '\n') {
                in_line_comment = false;
                if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
            }
        }
    }
}

bool has_specifier(
    const std::vector<parsed_specifier>& specifiers,
    const std::string_view name) {
    return std::any_of(specifiers.begin(), specifiers.end(),
        [name](const parsed_specifier& specifier) { return specifier.name == name; });
}

preprocessed_class_description& get_or_create_statics_class(
    const preprocessor_source& source,
    lexical_module_description& module) {
    const auto found = std::find_if(module.classes.begin(), module.classes.end(),
        [](const preprocessed_class_description& description) {
            return description.is_statics_class;
        });
    if (found != module.classes.end()) return *found;

    preprocessed_class_description description;
    description.class_name = "Module_" + make_identifier(module.module_name) + "Statics";
    description.super_class = "UObject";
    description.is_statics_class = true;
    std::string filename = source.relative_path;
    const std::size_t slash = filename.find_last_of("/\\");
    if (slash != std::string::npos) filename.erase(0U, slash + 1U);
    const std::size_t extension = filename.find_last_of('.');
    if (extension != std::string::npos) filename.erase(extension);
    set_metadata(description.metadata, "DisplayName", filename);
    module.statics_class_name = description.class_name;
    module.classes.push_back(std::move(description));
    return module.classes.back();
}

void process_function_macro(
    const preprocessor_options& options,
    const preprocessor_source& source,
    std::string& code,
    const reflection_macro& macro,
    preprocessed_class_description& description,
    lexical_preprocess_result& result,
    std::vector<text_replacement>& replacements,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices,
    bool& static_name_limit_exceeded,
    const bool editor_only) {
    preprocessed_function_description function;
    function.function_name = macro.name;
    function.script_function_name = macro.name;
    function.line = macro.line;
    function.is_static = description.is_statics_class;
    function.blueprint_callable = options.default_function_blueprint_callable;
    if (editor_only) set_metadata(function.metadata, "EditorOnly", "");
    if (description.is_struct) {
        add_diagnostic(
            result, source, macro.line,
            "Error parsing script struct " + description.class_name +
                ". Structs may not have any UFUNCTION()s.");
    }

    bool had_not_callable = false;
    bool had_callable = false;
    bool wrapper_generated = false;
    std::string wrapper;
    const std::vector<parsed_specifier> specifiers = parse_specifiers(macro.arguments);
    for (const parsed_specifier& specifier : specifiers) {
        const std::string& name = specifier.name;
        if (name == "BlueprintCallable") {
            function.blueprint_callable = true;
            had_callable = true;
        } else if (name == "NotBlueprintCallable") {
            function.blueprint_callable = false;
            had_not_callable = true;
        } else if (name == "BlueprintPure") {
            function.blueprint_callable = true;
            function.blueprint_pure = true;
        } else if (name == "BlueprintEvent") {
            if (function.is_static) {
                add_diagnostic(
                    result, source, macro.line,
                    "Global UFUNCTION() " + function.function_name +
                        " may not be marked BlueprintEvent.");
                continue;
            }
            if (function.blueprint_override) {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " cannot be both BlueprintEvent and BlueprintOverride.");
                continue;
            }
            const bool already_has_wrapper = function.blueprint_event;
            if (!had_callable) function.blueprint_callable = false;
            function.blueprint_event = true;
            function.can_override_event = true;
            if (!already_has_wrapper) {
                wrapper = generate_blueprint_event_wrapper(
                    options,
                    code,
                    macro,
                    function.function_name,
                    static_names,
                    static_name_indices,
                    static_name_limit_exceeded);
                wrapper_generated = true;
                function.script_function_name += "_Implementation";
            }
        } else if (options.angelscript_haze &&
                   (name == "NetFunction" || name == "CrumbFunction")) {
            if (function.is_static) {
                add_diagnostic(
                    result, source, macro.line,
                    "Static UFUNCTION()s cannot be NetFunction");
                continue;
            }
            if (function.blueprint_override) {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " cannot be both NetFunction and BlueprintOverride");
                continue;
            }
            const bool already_has_wrapper = function.blueprint_event;
            if (!had_not_callable) function.blueprint_callable = true;
            if (name == "CrumbFunction") set_metadata(function.metadata, name, "");
            function.blueprint_event = true;
            function.net_function = true;
            if (!already_has_wrapper) {
                function.can_override_event = false;
                wrapper = generate_blueprint_event_wrapper(
                    options,
                    code,
                    macro,
                    function.function_name,
                    static_names,
                    static_name_indices,
                    static_name_limit_exceeded);
                wrapper_generated = true;
                function.script_function_name += "_Implementation";
            }
        } else if (options.angelscript_haze && name == "DevFunction") {
            function.dev_function = true;
        } else if (!options.angelscript_haze &&
                   (name == "NetMulticast" || name == "Server" || name == "Client")) {
            if (function.blueprint_override) {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " cannot both be BlueprintOverride and have network specifiers");
                continue;
            }
            if (function.is_static) {
                add_diagnostic(
                    result, source, macro.line,
                    "Static UFUNCTION()s cannot use network specifiers");
                continue;
            }
            const bool already_has_wrapper = function.blueprint_event;
            if (!had_not_callable) function.blueprint_callable = true;
            function.blueprint_event = true;
            function.net_multicast = name == "NetMulticast";
            function.net_client = name == "Client";
            function.net_server = name == "Server";
            if (options.enforce_server_rpc_validation && function.net_server &&
                !has_specifier(specifiers, "WithValidation")) {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " is marked as Server but does not have the WithValidation property specified!");
                continue;
            }
            if (!already_has_wrapper) {
                function.can_override_event = false;
                wrapper = generate_blueprint_event_wrapper(
                    options,
                    code,
                    macro,
                    function.function_name,
                    static_names,
                    static_name_indices,
                    static_name_limit_exceeded);
                wrapper_generated = true;
                function.script_function_name += "_Implementation";
            }
        } else if (!options.angelscript_haze && name == "WithValidation") {
            if (has_specifier(specifiers, "Server") || has_specifier(specifiers, "Client")) {
                function.net_validate = true;
            } else {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " has the WithValidation property without the Server or Client property!");
            }
        } else if (!options.angelscript_haze && name == "BlueprintAuthorityOnly") {
            function.blueprint_authority_only = true;
        } else if (name == "Exec") {
            function.exec = true;
        } else if (name == "Unreliable") {
            function.unreliable = true;
        } else if (name == "BlueprintOverride") {
            if (function.is_static) {
                add_diagnostic(
                    result, source, macro.line,
                    "Global UFUNCTION() " + function.function_name +
                        " may not be BlueprintOverride.");
                continue;
            }
            if (function.blueprint_event) {
                add_diagnostic(
                    result, source, macro.line,
                    "UFUNCTION() " + function.function_name +
                        " cannot be both BlueprintEvent and BlueprintOverride.");
                continue;
            }
            if (!had_callable) function.blueprint_callable = false;
            function.blueprint_event = true;
            function.blueprint_override = true;
            function.script_function_name += "_Implementation";
        } else if (name == "CallInEditor") {
            set_metadata(function.metadata, name, "true");
        } else if (name == "Category" || name == "Keywords" || name == "ToolTip" ||
                   name == "DisplayName" || name == "BlueprintProtected") {
            set_metadata(function.metadata, name, specifier.value);
        } else if (name == "Meta") {
            for (const parsed_specifier& item : specifier.list) {
                if (!item.name.empty()) set_metadata(function.metadata, item.name, item.value);
            }
        } else if (name == "ForcedAssets") {
            std::string assets;
            for (const parsed_specifier& item : specifier.list) {
                if (!assets.empty()) assets += ';';
                assets += item.name + "=" + item.value;
            }
            set_metadata(function.metadata, name, assets);
        } else {
            add_diagnostic(
                result, source, macro.line,
                "Unknown function specifier " + name + " on method " +
                    description.class_name + "::" + function.script_function_name + ".");
        }
    }

    if (function.script_function_name != macro.name) {
        replacements.push_back({
            macro.name_start, macro.name_end, function.script_function_name});
    }
    if (wrapper_generated) {
        replacements.push_back({macro.start, macro.end, std::move(wrapper)});
    }
    replace_with_blank(code, macro.start, macro.end);
    description.methods.push_back(std::move(function));
}

void scan_function_macros_in_range(
    const preprocessor_options& options,
    const preprocessor_source& source,
    std::string& code,
    const std::size_t start,
    const std::size_t end,
    preprocessed_class_description& description,
    lexical_preprocess_result& result,
    std::vector<text_replacement>& replacements,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices,
    bool& static_name_limit_exceeded,
    const std::vector<std::vector<std::string>>& conditional_lines,
    const std::size_t class_condition_depth) {
    std::size_t depth = 1U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::uint32_t line = line_at(code, start);
    for (std::size_t position = start + 1U; position < end; ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (!in_comment && !in_string && depth == 1U && character == 'U' &&
            token_at(code, position, "UFUNCTION")) {
            const auto macro = parse_reflection_macro(
                code, position, end, reflection_macro_kind::function, line);
            if (macro.has_value()) {
                const bool editor_only = validate_reflection_macro_conditions(
                    options,
                    source,
                    conditional_lines,
                    macro->line,
                    class_condition_depth,
                    result);
                process_function_macro(
                    options,
                    source,
                    code,
                    *macro,
                    description,
                    result,
                    replacements,
                    static_names,
                    static_name_indices,
                    static_name_limit_exceeded,
                    editor_only);
                position = macro->end - 1U;
                continue;
            }
        }
        if (character == '/' && position + 1U < end && !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < end &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            in_string = !in_string;
        } else if (!in_comment && !in_string && character == '{') {
            ++depth;
        } else if (!in_comment && !in_string && character == '}' && depth > 0U) {
            --depth;
        } else if (character == '\n') {
            in_line_comment = false;
            if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
        }
    }
}

void process_function_macros(
    const preprocessor_options& options,
    const preprocessor_source& source,
    source_state& state,
    lexical_preprocess_result& result,
    std::vector<std::string>& static_names,
    std::unordered_map<std::string, std::size_t>& static_name_indices) {
    std::string& code = state.module.code.front().conditioned_code;
    std::vector<text_replacement> replacements;
    bool static_name_limit_exceeded = false;
    for (const source_type_range& range : state.type_ranges) {
        if (range.kind == source_type_kind::enum_type ||
            range.description_index >= state.module.classes.size() ||
            range.open >= range.close || range.close > code.size()) {
            continue;
        }
        scan_function_macros_in_range(
            options,
            source,
            code,
            range.open,
            range.close,
            state.module.classes[range.description_index],
            result,
            replacements,
            static_names,
            static_name_indices,
            static_name_limit_exceeded,
            state.conditional_lines,
            range.condition_depth);
    }

    std::vector<source_type_range> ranges = state.type_ranges;
    std::sort(ranges.begin(), ranges.end(),
        [](const source_type_range& left, const source_type_range& right) {
            return left.open < right.open;
        });
    std::size_t range_index = 0U;
    std::vector<std::string> namespace_stack;
    std::size_t scope_count = 0U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::uint32_t line = 1U;
    const auto top_level = [&]() { return scope_count <= namespace_stack.size(); };
    for (std::size_t position = 0U; position < code.size(); ++position) {
        while (range_index < ranges.size() && ranges[range_index].close < position) {
            ++range_index;
        }
        if (range_index < ranges.size() && position == ranges[range_index].open) {
            for (std::size_t scan = position; scan <= ranges[range_index].close; ++scan) {
                if (code[scan] == '\n' &&
                    line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
            }
            position = ranges[range_index].close;
            ++range_index;
            continue;
        }

        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (!in_comment && !in_string && top_level() && character == 'U' &&
            token_at(code, position, "UFUNCTION")) {
            const auto macro = parse_reflection_macro(
                code,
                position,
                code.size(),
                reflection_macro_kind::function,
                line);
            if (macro.has_value()) {
                const bool editor_only = validate_reflection_macro_conditions(
                    options,
                    source,
                    state.conditional_lines,
                    macro->line,
                    0U,
                    result);
                preprocessed_class_description& statics =
                    get_or_create_statics_class(source, state.module);
                process_function_macro(
                    options,
                    source,
                    code,
                    *macro,
                    statics,
                    result,
                    replacements,
                    static_names,
                    static_name_indices,
                    static_name_limit_exceeded,
                    editor_only);
                position = macro->end - 1U;
                continue;
            }
        } else if (!in_comment && !in_string && top_level() && character == 'n' &&
                   token_at(code, position, "namespace") &&
                   position + 9U < code.size() && is_whitespace(code[position + 9U])) {
            namespace_stack.push_back(read_identifier(code, position + 10U));
        }

        if (character == '/' && position + 1U < code.size() &&
            !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < code.size() &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            in_string = !in_string;
        } else if (!in_comment && !in_string && character == '{') {
            ++scope_count;
        } else if (!in_comment && !in_string && character == '}') {
            if (top_level() && !namespace_stack.empty()) namespace_stack.pop_back();
            if (scope_count > 0U) --scope_count;
        } else if (character == '\n') {
            in_line_comment = false;
            if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
        }
    }
    std::sort(replacements.begin(), replacements.end(),
        [](const text_replacement& left, const text_replacement& right) {
            return left.start < right.start;
        });
    adjust_type_ranges_for_replacements(state.type_ranges, replacements);
    code = apply_replacements(code, std::move(replacements));
    if (static_name_limit_exceeded) {
        add_diagnostic(
            result, source, 1U,
            "static-name table exceeds the bounded maximum");
    }
}

void refresh_defaults(source_state& state) {
    const std::string& code = state.module.code.front().conditioned_code;
    for (const source_type_range& range : state.type_ranges) {
        if (range.kind == source_type_kind::enum_type ||
            range.description_index >= state.module.classes.size() ||
            range.open >= range.close || range.close > code.size()) {
            continue;
        }
        state.module.classes[range.description_index].defaults_code =
            collect_class_defaults(code, range.open, range.close);
    }
}

std::size_t find_semicolon_directly_after(
    const std::string& code,
    const std::size_t position) noexcept {
    for (std::size_t scan = position + 1U; scan < code.size(); ++scan) {
        if (code[scan] == ';') return scan;
        if (!is_whitespace(code[scan])) return std::string::npos;
    }
    return std::string::npos;
}

std::string generate_delegate_code(
    const preprocessor_options& options,
    const std::string& code,
    const std::size_t name_start,
    const std::size_t name_end,
    const std::string& delegate_name,
    const bool multicast) {
    reflection_macro fake;
    fake.name_start = name_start;
    fake.name_end = name_end;
    const parsed_function_text function = parse_function_text(code, fake);
    std::unordered_set<std::string> specializations(
        options.blueprint_event_argument_specializations.begin(),
        options.blueprint_event_argument_specializations.end());
    std::string return_type = function.return_type;
    std::string qualified_return = return_type;
    if (function.const_return) qualified_return = "const " + qualified_return;
    if (starts_at(return_type, 0U, "delegate")) return_type.erase(0U, 8U);
    if (starts_at(return_type, 0U, "event")) return_type.erase(0U, 5U);

    std::string push_arguments;
    for (std::size_t index = 0U;
         index < function.argument_names.size() && index < function.argument_types.size();
         ++index) {
        const std::string& argument = function.argument_names[index];
        const std::string& type = function.argument_types[index];
        const bool reference = type.find('&') != std::string::npos;
        const bool const_reference = type.find("const ") != std::string::npos;
        if (reference && !const_reference) {
            push_arguments += " __Evt_PushArgumentRef" +
                push_argument_suffix(type, specializations) + "(" + argument + ");";
        } else {
            push_arguments += " __Evt_PushArgument" +
                push_argument_suffix(type, specializations) + "(" + argument + ");";
        }
    }

    std::string generated = "struct " + delegate_name + " {";
    generated += multicast
        ? "_FMulticastScriptDelegate _Inner;"
        : "_FScriptDelegate _Inner;";
    generated += delegate_name + "() __generated no_discard {}";
    generated += delegate_name + "(const " + delegate_name +
        "& Other) __generated no_discard { this = Other; }";
    generated += delegate_name + "& opAssign(const " + delegate_name +
        "& Other) __generated { _Inner = Other._Inner; return this; }";

    const bool has_return = return_type != "void";
    if (multicast) {
        generated += qualified_return + " Broadcast(" + function.arguments +
            ") const __generated {";
        generated += "if (!_Inner.IsBound()) return;";
        generated += push_arguments;
        generated += " __Evt_ExecuteDelegate(_Inner);}";
        generated += "void AddUFunction(const UObject Object, const FName& FunctionName) "
            "__generated { _Inner.AddUFunction(Object, FunctionName, "
            "__DelegateSignature(this)); }";
        generated += "void Unbind(UObject Object, const FName& FunctionName) "
            "__generated { _Inner.Unbind(Object, FunctionName); }";
        generated += "void UnbindObject(UObject Object) __generated { "
            "_Inner.UnbindObject(Object); }";
    } else {
        std::string generated_return;
        std::string generated_body = push_arguments;
        if (has_return) {
            generated_return = " " + return_type + " __ReturnValue" +
                return_initializer(return_type, options.script_float_is_float64) + ";";
            generated_body += "__Evt_PushArgumentRef" +
                push_argument_suffix(return_type, specializations) + "(__ReturnValue);";
        }
        generated_body += " __Evt_ExecuteDelegate(_Inner);";
        if (has_return) generated_body += " return __ReturnValue;";
        generated_body += "}";

        generated += qualified_return + " Execute(" + function.arguments +
            ") const allow_discard __generated {" + generated_return;
        generated += has_return
            ? "if (!_Inner.IsBound()) { Throw(\"Executing unbound delegate.\"); return __ReturnValue; }"
            : "if (!_Inner.IsBound()) { Throw(\"Executing unbound delegate.\"); return; }";
        generated += generated_body;
        generated += qualified_return + " ExecuteIfBound(" + function.arguments +
            ") const allow_discard __generated {" + generated_return;
        generated += has_return
            ? "if (!_Inner.IsBound()) { return __ReturnValue; }"
            : "if (!_Inner.IsBound()) { return; }";
        generated += generated_body;
        generated += "void BindUFunction(UObject Object, const FName& BindFunctionName) "
            "__generated { _Inner.BindUFunction(Object, BindFunctionName, "
            "__DelegateSignature(this)); }";
        generated += "UObject GetUObject() const property __generated { return "
            "_Inner.GetUObject(); }";
        generated += "FName GetFunctionName() const property __generated { return "
            "_Inner.GetFunctionName(); }";
        generated += delegate_name +
            "(UObject Object, const FName& BindFunctionName) __generated no_discard { "
            "_Inner.BindUFunction(Object, BindFunctionName, __DelegateSignature(this)); }";
    }
    generated += "bool IsBound() const __generated { return _Inner.IsBound(); }"
        "void Clear() __generated { _Inner.Clear(); }" "};";
    return generated;
}

void process_delegates(
    const preprocessor_options& options,
    const preprocessor_source& source,
    source_state& state,
    lexical_preprocess_result& result) {
    (void)source;
    (void)result;
    std::string& code = state.module.code.front().conditioned_code;
    state.module.delegates.clear();
    std::vector<std::string> generated;
    std::vector<std::string> namespace_stack;
    std::size_t scope_count = 0U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::uint32_t line = 1U;
    const auto top_level = [&]() { return scope_count <= namespace_stack.size(); };
    for (std::size_t position = 0U; position < code.size(); ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        bool multicast = false;
        std::size_t keyword_length = 0U;
        if (!in_comment && !in_string && top_level() && character == 'e' &&
            token_at(code, position, "event")) {
            multicast = true;
            keyword_length = 5U;
        } else if (!in_comment && !in_string && top_level() && character == 'd' &&
                   token_at(code, position, "delegate")) {
            keyword_length = 8U;
        }
        if (keyword_length != 0U) {
            const std::size_t open = code.find('(', position + keyword_length);
            const std::size_t close = open == std::string::npos
                ? std::string::npos
                : find_scope_close_parenthesis(code, open);
            const std::size_t semicolon = close == std::string::npos
                ? std::string::npos
                : find_semicolon_directly_after(code, close);
            if (open != std::string::npos && close != std::string::npos &&
                semicolon != std::string::npos) {
                std::size_t name_end = open;
                while (name_end > position && is_whitespace(code[name_end - 1U])) --name_end;
                std::size_t name_start = name_end;
                while (name_start > position) {
                    const char item = code[name_start - 1U];
                    if ((item >= 'a' && item <= 'z') || (item >= 'A' && item <= 'Z') ||
                        (item >= '0' && item <= '9') || item == '_') {
                        --name_start;
                    } else {
                        break;
                    }
                }
                const std::string name = code.substr(name_start, name_end - name_start);
                state.module.delegates.push_back({
                    name, joined_namespace(namespace_stack), line, multicast});
                generated.push_back(generate_delegate_code(
                    options, code, name_start, name_end, name, multicast));
                replace_with_blank(code, position, semicolon + 1U);
                position = semicolon;
                continue;
            }
        }

        if (!in_comment && !in_string && top_level() && character == 'n' &&
            token_at(code, position, "namespace") &&
            position + 9U < code.size() && is_whitespace(code[position + 9U])) {
            namespace_stack.push_back(read_identifier(code, position + 10U));
        }
        if (character == '/' && position + 1U < code.size() &&
            !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < code.size() &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            in_string = !in_string;
        } else if (!in_comment && !in_string && character == '{') {
            ++scope_count;
        } else if (!in_comment && !in_string && character == '}') {
            if (top_level() && !namespace_stack.empty()) namespace_stack.pop_back();
            if (scope_count > 0U) --scope_count;
        } else if (character == '\n') {
            in_line_comment = false;
            if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
        }
    }
    for (const std::string& item : generated) {
        code += "\n\n";
        code += item;
    }
}

std::string collect_class_defaults(
    const std::string& code,
    const std::size_t open,
    const std::size_t close) {
    std::string result;
    std::size_t depth = 1U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    for (std::size_t position = open + 1U; position < close; ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (character == '/' && position + 1U < close && !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
                continue;
            }
            if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
                continue;
            }
        } else if (character == '*' && position + 1U < close &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
            continue;
        } else if (character == '"' && !in_comment) {
            bool escaped = false;
            if (in_string) {
                std::size_t check = position;
                while (check > 0U && code[check - 1U] == '\\') {
                    escaped = !escaped;
                    --check;
                }
            }
            if (!escaped) in_string = !in_string;
        } else if (character == '\n') {
            in_line_comment = false;
        } else if (!in_comment && !in_string) {
            if (character == '{') {
                ++depth;
            } else if (character == '}' && depth > 0U) {
                --depth;
            } else if (depth == 1U && token_at(code, position, "default") &&
                       position + 7U < code.size() &&
                       is_whitespace(code[position + 7U])) {
                std::size_t end = position;
                while (end < close && code[end] != '\n') ++end;
                std::string line = trim(std::string_view(code).substr(
                    position + 8U, end - position - 8U));
                strip_comments_from_line(line);
                result += line;
                position = end == 0U ? end : end - 1U;
            }
        }
    }
    return result;
}

void collect_enum_meta(
    std::string& code,
    const std::size_t open,
    const std::size_t close,
    preprocessed_enum_description& description,
    const preprocessor_source& source,
    lexical_preprocess_result& result,
    std::uint32_t line) {
    std::int32_t value_index = 0;
    std::size_t parenthesis_depth = 0U;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    for (std::size_t position = open + 1U; position < close; ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (character == '/' && position + 1U < close && !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < close &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            in_string = !in_string;
        } else if (character == '\n') {
            in_line_comment = false;
            if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
        } else if (!in_comment && !in_string) {
            if (character == '(') {
                ++parenthesis_depth;
            } else if (character == ')' && parenthesis_depth > 0U) {
                --parenthesis_depth;
            } else if (character == ',' && parenthesis_depth == 0U) {
                ++value_index;
            } else if (character == 'U' && token_at(code, position, "UMETA")) {
                auto macro = parse_type_macro(
                    code,
                    position,
                    "UMETA(",
                    pending_type_macro::kind::enumeration,
                    line);
                if (macro.has_value() && macro->end <= close) {
                    apply_enum_specifiers(
                        *macro, description, source, result, value_index);
                    replace_with_blank(code, macro->start, macro->end);
                    position = macro->end - 1U;
                }
            }
        }
    }
}

std::string generated_static_class_code(
    const preprocessed_class_description& description,
    const static_class_mode mode) {
    if (description.is_struct) return {};
    const std::string& name = description.class_name;
    const std::string& variable = description.static_class_global_variable_name;
    if (mode == static_class_mode::disallowed) {
        if (!description.name_space.empty()) {
            return "namespace " + description.name_space +
                " { const TSubclassOf<UObject> " + variable + "; }";
        }
        return "const TSubclassOf<UObject> " + variable + ";";
    }
    const std::string suffix = mode == static_class_mode::deprecated ? " deprecated" : "";
    if (!description.name_space.empty()) {
        return "namespace " + description.name_space +
            " { const TSubclassOf<UObject> " + variable + "; namespace " + name +
            " { UClass StaticClass() __generated" + suffix + " { return " + variable +
            "; } } }";
    }
    return "const TSubclassOf<UObject> " + variable + "; namespace " + name +
        " { UClass StaticClass() __generated" + suffix + " { return " + variable +
        "; } }";
}

void analyze_declarations(
    const preprocessor_options& options,
    const preprocessor_source& source,
    lexical_module_description& module,
    std::vector<source_type_range>& type_ranges,
    const std::vector<std::vector<std::string>>& conditional_lines,
    lexical_preprocess_result& result,
    std::unordered_map<std::string, std::string>& declared_classes) {
    std::string& code = module.code.front().conditioned_code;
    static const std::regex class_pattern(
        R"((class|struct)\s+([A-Za-z0-9_]+)(\s*:\s*([A-Za-z0-9_]+\s*::\s*)*([A-Za-z0-9_]+))?)",
        std::regex::ECMAScript | std::regex::optimize);
    static const std::regex enum_pattern(
        R"((enum)\s+([A-Za-z0-9_]+))",
        std::regex::ECMAScript | std::regex::optimize);

    std::vector<std::string> namespace_stack;
    std::vector<std::string> generated_code;
    std::optional<pending_type_macro> pending_macro;
    bool in_line_comment = false;
    bool in_block_comment = false;
    bool in_string = false;
    std::size_t scope_count = 0U;
    std::uint32_t line = 1U;
    const auto top_level = [&]() { return scope_count <= namespace_stack.size(); };
    const auto condition_depth_at = [&conditional_lines](const std::uint32_t line_number) {
        const std::size_t index = static_cast<std::size_t>(line_number - 1U);
        return index < conditional_lines.size() ? conditional_lines[index].size() : 0U;
    };

    for (std::size_t position = 0U; position < code.size(); ++position) {
        const char character = code[position];
        const bool in_comment = in_line_comment || in_block_comment;
        if (!in_comment && !in_string && top_level() && character == 'U') {
            if (token_at(code, position, "UCLASS")) {
                pending_macro = parse_type_macro(
                    code, position, "UCLASS(",
                    pending_type_macro::kind::class_or_struct, line);
            } else if (token_at(code, position, "USTRUCT")) {
                pending_macro = parse_type_macro(
                    code, position, "USTRUCT(",
                    pending_type_macro::kind::class_or_struct, line);
            } else if (token_at(code, position, "UENUM")) {
                pending_macro = parse_type_macro(
                    code, position, "UENUM(",
                    pending_type_macro::kind::enumeration, line);
            }
            if (pending_macro.has_value()) {
                position = pending_macro->end - 1U;
                continue;
            }
        }

        if (!in_comment && !in_string && top_level() && character == 'c' &&
            token_at(code, position, "class")) {
            std::smatch match;
            const std::string tail = code.substr(position);
            if (std::regex_search(tail, match, class_pattern) && match.position() == 0) {
                preprocessed_class_description description;
                description.class_name = match[2].str();
                description.name_space = joined_namespace(namespace_stack);
                description.super_class = match[5].str();
                if (description.super_class.empty()) description.super_class = "UObject";
                description.static_class_global_variable_name =
                    "__StaticType_" + description.class_name;
                description.line = line;
                const std::size_t open = code.find('{', position + match.length());
                const std::size_t close = open == std::string::npos
                    ? std::string::npos
                    : find_scope_close_brace(code, open);
                if (open != std::string::npos && close != std::string::npos) {
                    description.defaults_code = collect_class_defaults(code, open, close);
                }
                if (pending_macro.has_value() &&
                    pending_macro->type == pending_type_macro::kind::class_or_struct) {
                    apply_class_specifiers(
                        *pending_macro, description, source, result);
                    replace_with_blank(code, pending_macro->start, pending_macro->end);
                }
                const auto inserted = declared_classes.emplace(
                    description.class_name, module.module_name);
                if (!inserted.second) {
                    add_diagnostic(
                        result,
                        source,
                        line,
                        "Cannot declare class " + description.class_name + " in module " +
                            module.module_name + ". A class with this name already exists in module " +
                            inserted.first->second + ".");
                }
                generated_code.push_back(generated_static_class_code(
                    description, options.static_classes));
                const std::size_t description_index = module.classes.size();
                module.classes.push_back(std::move(description));
                if (open != std::string::npos && close != std::string::npos) {
                    type_ranges.push_back({
                        source_type_kind::class_type,
                        open,
                        close,
                        description_index,
                        condition_depth_at(line),
                        position});
                }
                pending_macro.reset();
            }
        } else if (!in_comment && !in_string && top_level() && character == 's' &&
                   token_at(code, position, "struct")) {
            std::smatch match;
            const std::string tail = code.substr(position);
            if (std::regex_search(tail, match, class_pattern) && match.position() == 0) {
                preprocessed_class_description description;
                description.class_name = match[2].str();
                description.name_space = joined_namespace(namespace_stack);
                description.super_class = match[5].str();
                description.is_struct = true;
                description.line = line;
                const std::size_t open = code.find('{', position + match.length());
                const std::size_t close = open == std::string::npos
                    ? std::string::npos
                    : find_scope_close_brace(code, open);
                if (!description.super_class.empty()) {
                    add_diagnostic(
                        result,
                        source,
                        line,
                        "Error parsing script struct " + description.class_name +
                            ". Structs may not inherit from anything.");
                }
                if (open != std::string::npos && close != std::string::npos) {
                    description.defaults_code = collect_class_defaults(code, open, close);
                }
                if (pending_macro.has_value() &&
                    pending_macro->type == pending_type_macro::kind::class_or_struct) {
                    apply_class_specifiers(
                        *pending_macro, description, source, result);
                    replace_with_blank(code, pending_macro->start, pending_macro->end);
                }
                const auto inserted = declared_classes.emplace(
                    description.class_name, module.module_name);
                if (!inserted.second) {
                    add_diagnostic(
                        result,
                        source,
                        line,
                        "Cannot declare class " + description.class_name + " in module " +
                            module.module_name + ". A class with this name already exists in module " +
                            inserted.first->second + ".");
                }
                const std::size_t description_index = module.classes.size();
                module.classes.push_back(std::move(description));
                if (open != std::string::npos && close != std::string::npos) {
                    type_ranges.push_back({
                        source_type_kind::struct_type,
                        open,
                        close,
                        description_index,
                        condition_depth_at(line),
                        position});
                }
                pending_macro.reset();
            }
        } else if (!in_comment && !in_string && top_level() && character == 'e' &&
                   token_at(code, position, "enum")) {
            std::smatch match;
            const std::string tail = code.substr(position);
            if (std::regex_search(tail, match, enum_pattern) && match.position() == 0) {
                preprocessed_enum_description description;
                description.enum_name = match[2].str();
                description.name_space = joined_namespace(namespace_stack);
                description.line = line;
                if (pending_macro.has_value() &&
                    pending_macro->type == pending_type_macro::kind::enumeration) {
                    apply_enum_specifiers(
                        *pending_macro, description, source, result);
                    replace_with_blank(code, pending_macro->start, pending_macro->end);
                }
                const std::size_t open = code.find('{', position + match.length());
                const std::size_t close = open == std::string::npos
                    ? std::string::npos
                    : find_scope_close_brace(code, open);
                if (open != std::string::npos && close != std::string::npos) {
                    collect_enum_meta(
                        code, open, close, description, source, result, line);
                }
                const std::size_t description_index = module.enums.size();
                module.enums.push_back(std::move(description));
                if (open != std::string::npos && close != std::string::npos) {
                    type_ranges.push_back({
                        source_type_kind::enum_type,
                        open,
                        close,
                        description_index,
                        condition_depth_at(line),
                        position});
                }
                pending_macro.reset();
            }
        } else if (!in_comment && !in_string && top_level() && character == 'e' &&
                   token_at(code, position, "event")) {
            const std::size_t open = code.find('(', position + 5U);
            if (open != std::string::npos) {
                std::size_t end = open;
                while (end > position && is_whitespace(code[end - 1U])) --end;
                std::size_t start = end;
                while (start > position) {
                    const char item = code[start - 1U];
                    if ((item >= 'a' && item <= 'z') || (item >= 'A' && item <= 'Z') ||
                        (item >= '0' && item <= '9') || item == '_') {
                        --start;
                    } else {
                        break;
                    }
                }
                module.delegates.push_back({
                    code.substr(start, end - start), joined_namespace(namespace_stack), line, true});
            }
        } else if (!in_comment && !in_string && top_level() && character == 'd' &&
                   token_at(code, position, "delegate")) {
            const std::size_t open = code.find('(', position + 8U);
            if (open != std::string::npos) {
                std::size_t end = open;
                while (end > position && is_whitespace(code[end - 1U])) --end;
                std::size_t start = end;
                while (start > position) {
                    const char item = code[start - 1U];
                    if ((item >= 'a' && item <= 'z') || (item >= 'A' && item <= 'Z') ||
                        (item >= '0' && item <= '9') || item == '_') {
                        --start;
                    } else {
                        break;
                    }
                }
                module.delegates.push_back({
                    code.substr(start, end - start), joined_namespace(namespace_stack), line, false});
            }
        } else if (!in_comment && !in_string && top_level() && character == 'n' &&
                   token_at(code, position, "namespace") &&
                   position + 9U < code.size() && is_whitespace(code[position + 9U])) {
            namespace_stack.push_back(read_identifier(code, position + 10U));
        }

        if (character == '/' && position + 1U < code.size() &&
            !in_comment && !in_string) {
            if (code[position + 1U] == '/') {
                in_line_comment = true;
                ++position;
            } else if (code[position + 1U] == '*') {
                in_block_comment = true;
                ++position;
            }
        } else if (character == '*' && position + 1U < code.size() &&
                   in_block_comment && code[position + 1U] == '/') {
            in_block_comment = false;
            ++position;
        } else if (character == '"' && !in_comment) {
            bool escaped = false;
            if (in_string) {
                std::size_t check = position;
                while (check > 0U && code[check - 1U] == '\\') {
                    escaped = !escaped;
                    --check;
                }
            }
            if (!escaped) in_string = !in_string;
        } else if (character == '{' && !in_comment && !in_string) {
            ++scope_count;
        } else if (character == '}' && !in_comment && !in_string) {
            if (top_level() && !namespace_stack.empty()) namespace_stack.pop_back();
            if (scope_count > 0U) --scope_count;
        } else if (character == '\n') {
            in_line_comment = false;
            if (line != (std::numeric_limits<std::uint32_t>::max)()) ++line;
        }
    }

    for (const std::string& generated : generated_code) {
        if (generated.empty()) continue;
        code += "\n\n";
        code += generated;
    }
}

struct class_location {
    std::size_t state_index = 0U;
    std::size_t class_index = 0U;
    std::size_t range_index = 0U;
};

std::string generated_native_class_statics(
    const preprocessed_class_description& description) {
    const std::string qualified_namespace = description.name_space.empty()
        ? std::string{}
        : description.name_space + "::";
    const std::string prefix = "namespace " + qualified_namespace +
        description.class_name + " {";
    const std::string& name = description.class_name;
    const std::string& variable = description.static_class_global_variable_name;
    std::string generated = prefix;
    switch (description.code_super_kind) {
    case native_super_kind::actor:
        generated += "\n " + name +
            " Spawn(const FVector& Location = FVector::ZeroVector,"
            " const FRotator& Rotation = FRotator::ZeroRotator,"
            " const FName& Name = NAME_None, bool bDeferredSpawn = false, ULevel Level = nullptr) "
            "__generated {return Cast<" + name + ">(SpawnActor(" + variable +
            ".Get(), Location, Rotation, Name, bDeferredSpawn, Level));}";
        break;
    case native_super_kind::actor_component:
        generated += "\n " + name +
            " Get(const AActor Actor, FName WithName = NAME_None) __generated {" + name +
            " Value; __Actor_GetComponentByClass(Actor, " + variable +
            ", Value, WithName); return Value;}";
        generated += "\n " + name +
            " GetOrCreate(AActor Actor, FName WithName = NAME_None) __generated {" + name +
            " Value; __Actor_GetOrCreateComponentByClass(Actor, " + variable +
            ", Value, WithName); return Value;}";
        generated += "\n " + name +
            " Create(AActor Actor, FName WithName = NAME_None) __generated {" + name +
            " Value; __Actor_CreateComponentByClass(Actor, " + variable +
            ", Value, WithName); return Value;}";
        break;
    case native_super_kind::engine_subsystem:
        generated += "\n " + name + " Get() __generated {return Cast<" + name +
            ">(Subsystem::GetEngineSubsystem(" + variable + ".Get()));}";
        break;
    case native_super_kind::game_instance_subsystem:
        generated += "\n " + name + " Get() __generated {return Cast<" + name +
            ">(Subsystem::GetGameInstanceSubsystem(" + variable + ".Get()));}";
        break;
    case native_super_kind::world_subsystem:
        generated += "\n " + name + " Get() __generated {return Cast<" + name +
            ">(Subsystem::GetWorldSubsystem(" + variable + ".Get()));}";
        break;
    case native_super_kind::local_player_subsystem:
        generated += "\n " + name +
            " Get(ULocalPlayer LocalPlayer) __generated {return Cast<" + name +
            ">(Subsystem::GetLocalPlayerSubsystemFromLocalPlayer(LocalPlayer, " + variable +
            ".Get()));}";
        generated += "\n " + name +
            " Get(APlayerController PlayerController) __generated {return Cast<" + name +
            ">(Subsystem::GetLocalPlayerSubsystemFromPlayerController(PlayerController, " +
            variable + ".Get()));}";
        break;
    case native_super_kind::editor_subsystem:
    case native_super_kind::other_uobject:
        return {};
    }
    generated += "}";
    return generated;
}

void blank_native_inheritance(
    std::string& code,
    const source_type_range& range) {
    if (range.declaration_start >= range.open || range.open > code.size()) return;
    static const std::regex pattern(
        R"((class|struct)\s+([A-Za-z0-9_]+)(\s*:\s*[A-Za-z0-9_]+)?)",
        std::regex::ECMAScript | std::regex::optimize);
    const std::string declaration = code.substr(
        range.declaration_start, range.open - range.declaration_start);
    std::smatch match;
    if (!std::regex_search(declaration, match, pattern) || match.position() != 0 ||
        !match[3].matched) return;
    const std::size_t start = range.declaration_start +
        static_cast<std::size_t>(match.position(3));
    replace_with_blank(code, start, start + static_cast<std::size_t>(match.length(3)));
}

void resolve_class_hierarchy(
    const preprocessor_options& options,
    const std::vector<preprocessor_source>& sources,
    const std::vector<preprocessor_base_module>& base_modules,
    std::vector<source_state>& states,
    const std::vector<std::size_t>& order,
    lexical_preprocess_result& result) {
    std::unordered_map<std::string, const native_super_type*> native_types;
    std::unordered_map<std::string, native_super_kind> native_kinds_by_path;
    native_types.reserve(options.native_super_types.size());
    native_kinds_by_path.reserve(options.native_super_types.size());
    for (const native_super_type& type : options.native_super_types) {
        native_types.emplace(type.angelscript_type_name, &type);
        native_kinds_by_path.emplace(type.unreal_class_path, type.kind);
    }
    struct base_class_info {
        const preprocessor_base_class* description = nullptr;
        native_super_kind code_super_kind = native_super_kind::other_uobject;
    };
    std::unordered_set<std::string> edited_modules;
    for (const preprocessor_source& source : sources) {
        if (source.overlay_operation == preprocessor_source::operation::edit) {
            edited_modules.insert(effective_module_name(source));
        }
    }
    std::unordered_map<std::string, base_class_info> base_classes;
    for (const preprocessor_base_module& module : base_modules) {
        if (edited_modules.find(module.module_name) != edited_modules.end()) continue;
        for (const preprocessor_base_class& type : module.classes) {
            if (type.is_struct) continue;
            const auto kind = native_kinds_by_path.find(type.code_super_class);
            if (kind == native_kinds_by_path.end()) {
                result.diagnostics.push_back({
                    preprocessor_diagnostic_severity::error, {}, 1U, 1U,
                    "base class " + type.class_name +
                        " has an unprofiled native code superclass " +
                        type.code_super_class});
                continue;
            }
            base_classes.emplace(type.class_name, base_class_info{&type, kind->second});
        }
    }
    std::unordered_map<std::string, class_location> classes;
    for (std::size_t state_index = 0U; state_index < states.size(); ++state_index) {
        for (std::size_t range_index = 0U;
             range_index < states[state_index].type_ranges.size();
             ++range_index) {
            const source_type_range& range = states[state_index].type_ranges[range_index];
            if (range.kind == source_type_kind::enum_type ||
                range.description_index >= states[state_index].module.classes.size()) continue;
            const auto& description =
                states[state_index].module.classes[range.description_index];
            if (base_classes.find(description.class_name) != base_classes.end()) {
                add_diagnostic(
                    result,
                    sources[state_index],
                    description.line,
                    "Class " + description.class_name +
                        " collides with a class in an unchanged base module.");
            }
            classes.emplace(
                description.class_name,
                class_location{state_index, range.description_index, range_index});
        }
    }

    enum class visit_state { resolving, resolved, failed };
    std::unordered_map<std::string, visit_state> visits;
    const auto resolve = [&](const auto& self, const class_location location) -> bool {
        preprocessed_class_description& description =
            states[location.state_index].module.classes[location.class_index];
        if (description.is_struct) return true;
        const auto visited = visits.find(description.class_name);
        if (visited != visits.end()) {
            return visited->second == visit_state::resolved;
        }
        visits.emplace(description.class_name, visit_state::resolving);
        const auto native = native_types.find(description.super_class);
        if (native != native_types.end()) {
            description.super_is_code_class = true;
            description.code_super_class = native->second->unreal_class_path;
            description.code_super_kind = native->second->kind;
            if (native->second->cannot_derive_angelscript) {
                add_diagnostic(
                    result,
                    sources[location.state_index],
                    description.line,
                    "Class " + description.class_name + " cannot inherit from C++ class " +
                        description.super_class +
                        " which specifies CannotDeriveAngelscript meta");
                visits[description.class_name] = visit_state::failed;
                return false;
            }
            blank_native_inheritance(
                states[location.state_index].module.code.front().conditioned_code,
                states[location.state_index].type_ranges[location.range_index]);
        } else {
            const auto parent = classes.find(description.super_class);
            const auto base_parent = base_classes.find(description.super_class);
            if (base_parent != base_classes.end()) {
                description.code_super_class =
                    base_parent->second.description->code_super_class;
                description.code_super_kind = base_parent->second.code_super_kind;
            } else if (parent == classes.end() ||
                (visits.find(description.super_class) != visits.end() &&
                 visits[description.super_class] == visit_state::resolving) ||
                !self(self, parent->second)) {
                add_diagnostic(
                    result,
                    sources[location.state_index],
                    description.line,
                    "Class " + description.class_name + " has an unknown super type " +
                        description.super_class + ".");
                visits[description.class_name] = visit_state::failed;
                return false;
            } else {
                const preprocessed_class_description& parent_description =
                    states[parent->second.state_index].module.classes[parent->second.class_index];
                description.code_super_class = parent_description.code_super_class;
                description.code_super_kind = parent_description.code_super_kind;
            }
        }
        visits[description.class_name] = visit_state::resolved;
        return true;
    };

    for (const std::size_t state_index : order) {
        for (std::size_t range_index = 0U;
             range_index < states[state_index].type_ranges.size();
             ++range_index) {
            const source_type_range& range = states[state_index].type_ranges[range_index];
            if (range.kind == source_type_kind::enum_type ||
                range.description_index >= states[state_index].module.classes.size()) continue;
            const class_location location{state_index, range.description_index, range_index};
            resolve(resolve, location);
        }
    }
    for (const std::size_t state_index : order) {
        for (const source_type_range& range : states[state_index].type_ranges) {
            if (range.kind == source_type_kind::enum_type ||
                range.description_index >= states[state_index].module.classes.size()) continue;
            const preprocessed_class_description& description =
                states[state_index].module.classes[range.description_index];
            if (visits.find(description.class_name) == visits.end() ||
                visits[description.class_name] != visit_state::resolved) continue;
            const std::string generated = generated_native_class_statics(description);
            if (!generated.empty()) {
                std::string& code =
                    states[state_index].module.code.front().conditioned_code;
                code += "\n\n";
                code += generated;
            }
        }
    }
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

    state.conditional_lines.clear();
    state.conditional_lines.push_back({});

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
            if (line_number != (std::numeric_limits<std::uint32_t>::max)()) {
                ++line_number;
                std::vector<std::string> conditions;
                conditions.reserve(ifdefs.size());
                for (const active_ifdef& active : ifdefs) {
                    conditions.push_back(active.condition);
                }
                state.conditional_lines.push_back(std::move(conditions));
            }
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
    const std::vector<preprocessor_source>& sources,
    const std::vector<preprocessor_base_module>& base_modules) {
    lexical_preprocess_result result;
    if (!validate_inputs(options, sources, base_modules, result)) return result;

    std::unordered_map<std::string, bool> flags;
    flags.reserve(options.flags.size());
    for (const preprocessor_flag& flag : options.flags) flags.emplace(flag.name, flag.value);

    result.static_names = options.static_names;
    std::unordered_map<std::string, std::size_t> static_name_indices;
    static_name_indices.reserve(result.static_names.size());
    for (std::size_t index = 0U; index < result.static_names.size(); ++index) {
        static_name_indices.emplace(ascii_fold(result.static_names[index]), index);
    }

    std::vector<source_state> states;
    states.reserve(sources.size());
    std::size_t total_imports = 0U;
    for (const preprocessor_source& source : sources) {
        source_state state;
        state.module.module_name = effective_module_name(source);
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

    std::unordered_map<std::string, std::string> declared_classes;
    declared_classes.reserve(states.size());
    for (const std::size_t index : order) {
        analyze_declarations(
            options,
            sources[index],
            states[index].module,
            states[index].type_ranges,
            states[index].conditional_lines,
            result,
            declared_classes);
    }
    resolve_class_hierarchy(options, sources, base_modules, states, order, result);

    // Name/F-string stream replacements are discovered in original Files
    // order before reflection macros can append wrapper static names.
    for (std::size_t index = 0U; index < states.size(); ++index) {
        bool static_name_limit_exceeded = false;
        std::string& code = states[index].module.code.front().conditioned_code;
        lower_name_and_format_literals(
            code,
            result.static_names,
            static_name_indices,
            static_name_limit_exceeded,
            &states[index].type_ranges);
        if (static_name_limit_exceeded) {
            add_diagnostic(
                result, sources[index], 1U,
                "static-name table exceeds the bounded maximum");
        }
    }

    for (const std::size_t index : order) {
        process_property_macros(options, sources[index], states[index], result);
    }

    for (const std::size_t index : order) {
        process_function_macros(
            options,
            sources[index],
            states[index],
            result,
            result.static_names,
            static_name_indices);
        refresh_defaults(states[index]);
    }

    for (const std::size_t index : order) {
        process_delegates(options, sources[index], states[index], result);
    }

    // Donor post-processing runs in dependency-sorted order.
    for (const std::size_t index : order) {
        std::string& code = states[index].module.code.front().conditioned_code;
        lower_range_based_for(code);
        lower_literal_assets(code, states[index].module.post_init_functions);
        if (states[index].module.post_init_functions.size() >
            max_preprocessor_post_init_functions) {
            add_diagnostic(
                result, sources[index], 1U,
                "post-init function count exceeds the bounded maximum");
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
