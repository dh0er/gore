#include "gore_as_standalone/json.hpp"

#include <algorithm>
#include <charconv>
#include <exception>
#include <limits>
#include <set>

namespace gore::as::standalone::json {
namespace {

bool valid_utf8(std::string_view text) noexcept {
    std::size_t index = 0U;
    while (index < text.size()) {
        const auto first = static_cast<unsigned char>(text[index]);
        if (first <= 0x7fU) {
            ++index;
            continue;
        }
        std::size_t count = 0U;
        std::uint32_t code_point = 0U;
        std::uint32_t minimum = 0U;
        if ((first & 0xe0U) == 0xc0U) {
            count = 1U; code_point = first & 0x1fU; minimum = 0x80U;
        } else if ((first & 0xf0U) == 0xe0U) {
            count = 2U; code_point = first & 0x0fU; minimum = 0x800U;
        } else if ((first & 0xf8U) == 0xf0U) {
            count = 3U; code_point = first & 0x07U; minimum = 0x10000U;
        } else {
            return false;
        }
        if (count > text.size() - index - 1U) return false;
        for (std::size_t offset = 1U; offset <= count; ++offset) {
            const auto next = static_cast<unsigned char>(text[index + offset]);
            if ((next & 0xc0U) != 0x80U) return false;
            code_point = (code_point << 6U) | (next & 0x3fU);
        }
        if (code_point < minimum || code_point > 0x10ffffU ||
            (code_point >= 0xd800U && code_point <= 0xdfffU)) return false;
        index += count + 1U;
    }
    return true;
}

void append_utf8(const std::uint32_t code_point, std::string& output) {
    if (code_point <= 0x7fU) {
        output.push_back(static_cast<char>(code_point));
    } else if (code_point <= 0x7ffU) {
        output.push_back(static_cast<char>(0xc0U | (code_point >> 6U)));
        output.push_back(static_cast<char>(0x80U | (code_point & 0x3fU)));
    } else if (code_point <= 0xffffU) {
        output.push_back(static_cast<char>(0xe0U | (code_point >> 12U)));
        output.push_back(static_cast<char>(0x80U | ((code_point >> 6U) & 0x3fU)));
        output.push_back(static_cast<char>(0x80U | (code_point & 0x3fU)));
    } else {
        output.push_back(static_cast<char>(0xf0U | (code_point >> 18U)));
        output.push_back(static_cast<char>(0x80U | ((code_point >> 12U) & 0x3fU)));
        output.push_back(static_cast<char>(0x80U | ((code_point >> 6U) & 0x3fU)));
        output.push_back(static_cast<char>(0x80U | (code_point & 0x3fU)));
    }
}

class parser final {
public:
    parser(const std::string_view input, const std::size_t max_depth) noexcept
        : input_(input), max_depth_(max_depth) {}

    bool run(value& output, parse_error& error) {
        if (!valid_utf8(input_)) return fail(error, "input is not canonical UTF-8");
        skip_space();
        if (!parse_value(0U, output, error)) return false;
        skip_space();
        if (position_ != input_.size()) return fail(error, "trailing bytes after JSON value");
        return true;
    }

private:
    bool fail(parse_error& error, std::string detail) const {
        error.offset = position_;
        error.detail = std::move(detail);
        return false;
    }

    void skip_space() noexcept {
        while (position_ < input_.size() &&
            (input_[position_] == ' ' || input_[position_] == '\t' ||
             input_[position_] == '\r' || input_[position_] == '\n')) ++position_;
    }

    bool parse_value(const std::size_t depth, value& output, parse_error& error) {
        if (depth > max_depth_) return fail(error, "JSON nesting limit exceeded");
        if (position_ >= input_.size()) return fail(error, "unexpected end of JSON");
        const char ch = input_[position_];
        if (ch == '{') return parse_object(depth, output, error);
        if (ch == '[') return parse_array(depth, output, error);
        if (ch == '"') {
            output.kind = value_kind::string;
            return parse_string(output.text, error);
        }
        if (ch == 't' && take("true")) {
            output.kind = value_kind::boolean; output.boolean = true; return true;
        }
        if (ch == 'f' && take("false")) {
            output.kind = value_kind::boolean; output.boolean = false; return true;
        }
        if (ch == 'n' && take("null")) {
            output.kind = value_kind::null_value; return true;
        }
        return parse_number(output, error);
    }

    bool take(const std::string_view token) noexcept {
        if (input_.substr(position_, token.size()) != token) return false;
        position_ += token.size();
        return true;
    }

    static int hex_value(const char ch) noexcept {
        if (ch >= '0' && ch <= '9') return ch - '0';
        if (ch >= 'a' && ch <= 'f') return ch - 'a' + 10;
        if (ch >= 'A' && ch <= 'F') return ch - 'A' + 10;
        return -1;
    }

    bool code_unit(std::uint32_t& output, parse_error& error) {
        if (input_.size() - position_ < 4U) return fail(error, "truncated JSON unicode escape");
        output = 0U;
        for (std::size_t index = 0U; index < 4U; ++index) {
            const int digit = hex_value(input_[position_++]);
            if (digit < 0) return fail(error, "invalid JSON unicode escape");
            output = (output << 4U) | static_cast<std::uint32_t>(digit);
        }
        return true;
    }

    bool parse_string(std::string& output, parse_error& error) {
        if (input_[position_++] != '"') return fail(error, "expected JSON string");
        output.clear();
        while (position_ < input_.size()) {
            const auto ch = static_cast<unsigned char>(input_[position_++]);
            if (ch == '"') return true;
            if (ch < 0x20U) return fail(error, "unescaped control character in JSON string");
            if (ch != '\\') {
                output.push_back(static_cast<char>(ch));
                continue;
            }
            if (position_ >= input_.size()) return fail(error, "truncated JSON escape");
            const char escaped = input_[position_++];
            switch (escaped) {
            case '"': output.push_back('"'); break;
            case '\\': output.push_back('\\'); break;
            case '/': output.push_back('/'); break;
            case 'b': output.push_back('\b'); break;
            case 'f': output.push_back('\f'); break;
            case 'n': output.push_back('\n'); break;
            case 'r': output.push_back('\r'); break;
            case 't': output.push_back('\t'); break;
            case 'u': {
                std::uint32_t first = 0U;
                if (!code_unit(first, error)) return false;
                if (first >= 0xd800U && first <= 0xdbffU) {
                    if (input_.size() - position_ < 6U || input_[position_] != '\\' ||
                        input_[position_ + 1U] != 'u') {
                        return fail(error, "unpaired high surrogate in JSON string");
                    }
                    position_ += 2U;
                    std::uint32_t second = 0U;
                    if (!code_unit(second, error)) return false;
                    if (second < 0xdc00U || second > 0xdfffU) {
                        return fail(error, "invalid low surrogate in JSON string");
                    }
                    append_utf8(0x10000U + ((first - 0xd800U) << 10U) + (second - 0xdc00U), output);
                } else if (first >= 0xdc00U && first <= 0xdfffU) {
                    return fail(error, "unpaired low surrogate in JSON string");
                } else {
                    append_utf8(first, output);
                }
                break;
            }
            default: return fail(error, "unknown JSON escape");
            }
        }
        return fail(error, "unterminated JSON string");
    }

    bool parse_number(value& output, parse_error& error) {
        const std::size_t begin = position_;
        if (position_ < input_.size() && input_[position_] == '-') ++position_;
        if (position_ >= input_.size()) return fail(error, "truncated JSON number");
        if (input_[position_] == '0') {
            ++position_;
            if (position_ < input_.size() && input_[position_] >= '0' && input_[position_] <= '9') {
                return fail(error, "leading zero in JSON number");
            }
        } else if (input_[position_] >= '1' && input_[position_] <= '9') {
            do { ++position_; } while (position_ < input_.size() &&
                input_[position_] >= '0' && input_[position_] <= '9');
        } else {
            return fail(error, "invalid JSON value");
        }
        if (position_ < input_.size() &&
            (input_[position_] == '.' || input_[position_] == 'e' || input_[position_] == 'E')) {
            return fail(error, "floating-point JSON number is not allowed in this protocol");
        }
        output.kind = value_kind::number;
        output.text.assign(input_.substr(begin, position_ - begin));
        return true;
    }

    bool parse_array(const std::size_t depth, value& output, parse_error& error) {
        ++position_;
        output.kind = value_kind::array;
        output.elements.clear();
        skip_space();
        if (position_ < input_.size() && input_[position_] == ']') { ++position_; return true; }
        for (;;) {
            value element;
            if (!parse_value(depth + 1U, element, error)) return false;
            output.elements.push_back(std::move(element));
            skip_space();
            if (position_ >= input_.size()) return fail(error, "unterminated JSON array");
            const char delimiter = input_[position_++];
            if (delimiter == ']') return true;
            if (delimiter != ',') return fail(error, "expected comma in JSON array");
            skip_space();
        }
    }

    bool parse_object(const std::size_t depth, value& output, parse_error& error) {
        ++position_;
        output.kind = value_kind::object;
        output.members.clear();
        std::set<std::string> names;
        skip_space();
        if (position_ < input_.size() && input_[position_] == '}') { ++position_; return true; }
        for (;;) {
            if (position_ >= input_.size() || input_[position_] != '"') {
                return fail(error, "expected string key in JSON object");
            }
            std::string name;
            if (!parse_string(name, error)) return false;
            if (!names.insert(name).second) return fail(error, "duplicate JSON object key");
            skip_space();
            if (position_ >= input_.size() || input_[position_++] != ':') {
                return fail(error, "expected colon in JSON object");
            }
            skip_space();
            value member;
            if (!parse_value(depth + 1U, member, error)) return false;
            output.members.emplace_back(std::move(name), std::move(member));
            skip_space();
            if (position_ >= input_.size()) return fail(error, "unterminated JSON object");
            const char delimiter = input_[position_++];
            if (delimiter == '}') return true;
            if (delimiter != ',') return fail(error, "expected comma in JSON object");
            skip_space();
        }
    }

    std::string_view input_;
    std::size_t max_depth_ = 0U;
    std::size_t position_ = 0U;
};

void append_escaped(const std::string_view text, std::string& output) {
    constexpr char hex[] = "0123456789abcdef";
    output.push_back('"');
    for (const unsigned char ch : text) {
        switch (ch) {
        case '"': output += "\\\""; break;
        case '\\': output += "\\\\"; break;
        case '\b': output += "\\b"; break;
        case '\f': output += "\\f"; break;
        case '\n': output += "\\n"; break;
        case '\r': output += "\\r"; break;
        case '\t': output += "\\t"; break;
        default:
            if (ch < 0x20U) {
                output += "\\u00";
                output.push_back(hex[(ch >> 4U) & 0x0fU]);
                output.push_back(hex[ch & 0x0fU]);
            } else {
                output.push_back(static_cast<char>(ch));
            }
        }
    }
    output.push_back('"');
}

void serialize(const value& input, std::string& output) {
    switch (input.kind) {
    case value_kind::null_value: output += "null"; break;
    case value_kind::boolean: output += input.boolean ? "true" : "false"; break;
    case value_kind::number: output += input.text; break;
    case value_kind::string: append_escaped(input.text, output); break;
    case value_kind::array:
        output.push_back('[');
        for (std::size_t index = 0U; index < input.elements.size(); ++index) {
            if (index != 0U) output.push_back(',');
            serialize(input.elements[index], output);
        }
        output.push_back(']');
        break;
    case value_kind::object:
        output.push_back('{');
        for (std::size_t index = 0U; index < input.members.size(); ++index) {
            if (index != 0U) output.push_back(',');
            append_escaped(input.members[index].first, output);
            output.push_back(':');
            serialize(input.members[index].second, output);
        }
        output.push_back('}');
        break;
    }
}

bool named_value(
    const value& input,
    const std::string_view name,
    const value_kind kind,
    const value*& output,
    std::string& detail) {
    if (input.kind != value_kind::object) {
        detail = "expected object while reading field " + std::string(name);
        return false;
    }
    output = input.find(name);
    if (output == nullptr) {
        detail = "missing required field " + std::string(name);
        return false;
    }
    if (output->kind != kind) {
        detail = "field " + std::string(name) + " has the wrong JSON type";
        return false;
    }
    return true;
}

} // namespace

const value* value::find(const std::string_view name) const noexcept {
    if (kind != value_kind::object) return nullptr;
    for (const auto& member : members) if (member.first == name) return &member.second;
    return nullptr;
}

bool parse(
    const std::string_view input,
    const std::size_t max_depth,
    value& output,
    parse_error& error) noexcept {
    try {
        value staged;
        parse_error staged_error;
        parser reader(input, max_depth);
        if (!reader.run(staged, staged_error)) { error = std::move(staged_error); return false; }
        output = std::move(staged);
        error = {};
        return true;
    } catch (const std::exception& exception) {
        error.detail = exception.what();
        return false;
    } catch (...) {
        error.detail = "unknown JSON parser failure";
        return false;
    }
}

bool require_object_keys(
    const value& input,
    const std::initializer_list<std::string_view> required,
    const std::initializer_list<std::string_view> optional,
    std::string& detail) {
    if (input.kind != value_kind::object) { detail = "expected JSON object"; return false; }
    for (const auto name : required) {
        if (input.find(name) == nullptr) { detail = "missing required field " + std::string(name); return false; }
    }
    for (const auto& member : input.members) {
        const auto allowed = [&](const std::string_view name) { return member.first == name; };
        if (std::none_of(required.begin(), required.end(), allowed) &&
            std::none_of(optional.begin(), optional.end(), allowed)) {
            detail = "unknown field " + member.first;
            return false;
        }
    }
    return true;
}

bool get_object(const value& input, const std::string_view name, const value*& output, std::string& detail) {
    return named_value(input, name, value_kind::object, output, detail);
}
bool get_array(const value& input, const std::string_view name, const value*& output, std::string& detail) {
    return named_value(input, name, value_kind::array, output, detail);
}
bool get_string(const value& input, const std::string_view name, std::string& output, std::string& detail) {
    const value* member = nullptr;
    if (!named_value(input, name, value_kind::string, member, detail)) return false;
    output = member->text;
    return true;
}
bool get_bool(const value& input, const std::string_view name, bool& output, std::string& detail) {
    const value* member = nullptr;
    if (!named_value(input, name, value_kind::boolean, member, detail)) return false;
    output = member->boolean;
    return true;
}
bool get_u64(const value& input, const std::string_view name, std::uint64_t& output, std::string& detail) {
    const value* member = nullptr;
    if (!named_value(input, name, value_kind::number, member, detail)) return false;
    if (member->text.empty() || member->text.front() == '-') {
        detail = "field " + std::string(name) + " is not an unsigned integer";
        return false;
    }
    const auto result = std::from_chars(member->text.data(), member->text.data() + member->text.size(), output);
    if (result.ec != std::errc{} || result.ptr != member->text.data() + member->text.size()) {
        detail = "field " + std::string(name) + " is outside uint64";
        return false;
    }
    return true;
}
bool get_i64(const value& input, const std::string_view name, std::int64_t& output, std::string& detail) {
    const value* member = nullptr;
    if (!named_value(input, name, value_kind::number, member, detail)) return false;
    const auto result = std::from_chars(member->text.data(), member->text.data() + member->text.size(), output);
    if (result.ec != std::errc{} || result.ptr != member->text.data() + member->text.size()) {
        detail = "field " + std::string(name) + " is outside int64";
        return false;
    }
    return true;
}
bool get_optional_u64(
    const value& input,
    const std::string_view name,
    bool& present,
    std::uint64_t& output,
    std::string& detail) {
    const value* member = input.find(name);
    if (member == nullptr || member->kind == value_kind::null_value) { present = false; return true; }
    present = true;
    return get_u64(input, name, output, detail);
}
bool get_optional_string(
    const value& input,
    const std::string_view name,
    bool& present,
    std::string& output,
    std::string& detail) {
    const value* member = input.find(name);
    if (member == nullptr || member->kind == value_kind::null_value) { present = false; return true; }
    present = true;
    return get_string(input, name, output, detail);
}
bool serialize_compact(const value& input, std::string& output) noexcept {
    try {
        std::string staged;
        serialize(input, staged);
        output = std::move(staged);
        return true;
    } catch (...) {
        return false;
    }
}

} // namespace gore::as::standalone::json
