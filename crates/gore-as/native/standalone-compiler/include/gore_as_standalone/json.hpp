#pragma once

#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace gore::as::standalone::json {

enum class value_kind { null_value, boolean, number, string, array, object };

struct value {
    value_kind kind = value_kind::null_value;
    bool boolean = false;
    std::string text;
    std::vector<value> elements;
    std::vector<std::pair<std::string, value>> members;

    [[nodiscard]] const value* find(std::string_view name) const noexcept;
};

struct parse_error {
    std::size_t offset = 0U;
    std::string detail;
};

// Strict RFC 8259 JSON parser. Duplicate object keys, invalid UTF-8, invalid
// surrogate pairs and non-integer numeric spellings are rejected. Protocol
// payloads deliberately have no floating-point fields.
bool parse(
    std::string_view input,
    std::size_t max_depth,
    value& output,
    parse_error& error) noexcept;

bool require_object_keys(
    const value& input,
    std::initializer_list<std::string_view> required,
    std::initializer_list<std::string_view> optional,
    std::string& detail);

bool get_object(const value& input, std::string_view name, const value*& output, std::string& detail);
bool get_array(const value& input, std::string_view name, const value*& output, std::string& detail);
bool get_string(const value& input, std::string_view name, std::string& output, std::string& detail);
bool get_bool(const value& input, std::string_view name, bool& output, std::string& detail);
bool get_u64(const value& input, std::string_view name, std::uint64_t& output, std::string& detail);
bool get_i64(const value& input, std::string_view name, std::int64_t& output, std::string& detail);
bool get_optional_u64(
    const value& input,
    std::string_view name,
    bool& present,
    std::uint64_t& output,
    std::string& detail);
bool get_optional_string(
    const value& input,
    std::string_view name,
    bool& present,
    std::string& output,
    std::string& detail);

// Compact serialization preserves object member order. It is used only for
// the manifest's domain-separated canonical identity, whose order is fixed by
// the Rust schema.
bool serialize_compact(const value& input, std::string& output) noexcept;

} // namespace gore::as::standalone::json
