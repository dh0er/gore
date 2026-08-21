#include "gore_as_standalone/precompiled_data.hpp"

#include <algorithm>
#include <cstring>
#include <exception>
#include <limits>
#include <new>
#include <type_traits>
#include <unordered_set>

namespace gore::as::standalone::precompiled {
namespace {

constexpr std::size_t kDataTypeBytes = 36U;
constexpr std::size_t kMinimumFunctionBytes = 120U;
constexpr std::size_t kMinimumPropertyBytes = 52U;
constexpr std::size_t kMinimumClassBytes = 64U;
constexpr std::size_t kMinimumEnumBytes = 16U;
constexpr std::size_t kMinimumGlobalBytes = 48U;
constexpr std::size_t kMinimumImportBytes = 60U;
constexpr std::size_t kMinimumModulePairBytes = 60U;

[[nodiscard]] bool valid_utf16_payload(const std::uint8_t* payload, const std::size_t size) {
    if ((size % 2U) != 0U) {
        return false;
    }
    for (std::size_t offset = 0U; offset < size; offset += 2U) {
        const std::uint16_t unit = static_cast<std::uint16_t>(payload[offset]) |
            (static_cast<std::uint16_t>(payload[offset + 1U]) << 8U);
        if (unit == 0U) {
            return false;
        }
        if (unit >= 0xd800U && unit <= 0xdbffU) {
            if (offset + 3U >= size) {
                return false;
            }
            const std::uint16_t low = static_cast<std::uint16_t>(payload[offset + 2U]) |
                (static_cast<std::uint16_t>(payload[offset + 3U]) << 8U);
            if (low < 0xdc00U || low > 0xdfffU) {
                return false;
            }
            offset += 2U;
        } else if (unit >= 0xdc00U && unit <= 0xdfffU) {
            return false;
        }
    }
    return true;
}

[[nodiscard]] bool valid_utf8(const std::string& text) noexcept {
    std::size_t index = 0U;
    while (index < text.size()) {
        const auto first = static_cast<unsigned char>(text[index]);
        if (first <= 0x7fU) {
            ++index;
            continue;
        }
        std::size_t continuation_count = 0U;
        std::uint32_t code_point = 0U;
        std::uint32_t minimum = 0U;
        if ((first & 0xe0U) == 0xc0U) {
            continuation_count = 1U;
            code_point = first & 0x1fU;
            minimum = 0x80U;
        } else if ((first & 0xf0U) == 0xe0U) {
            continuation_count = 2U;
            code_point = first & 0x0fU;
            minimum = 0x800U;
        } else if ((first & 0xf8U) == 0xf0U) {
            continuation_count = 3U;
            code_point = first & 0x07U;
            minimum = 0x10000U;
        } else {
            return false;
        }
        if (continuation_count > text.size() - index - 1U) {
            return false;
        }
        for (std::size_t offset = 1U; offset <= continuation_count; ++offset) {
            const auto next = static_cast<unsigned char>(text[index + offset]);
            if ((next & 0xc0U) != 0x80U) {
                return false;
            }
            code_point = (code_point << 6U) | (next & 0x3fU);
        }
        if (code_point < minimum || code_point > 0x10ffffU ||
            (code_point >= 0xd800U && code_point <= 0xdfffU)) {
            return false;
        }
        index += continuation_count + 1U;
    }
    return true;
}

class reader final {
public:
    reader(const std::uint8_t* bytes, const std::size_t size, codec_error& error) noexcept
        : bytes_(bytes), size_(size), error_(error) {}

    [[nodiscard]] std::size_t position() const noexcept { return position_; }
    [[nodiscard]] std::size_t remaining() const noexcept { return size_ - position_; }
    [[nodiscard]] bool failed() const noexcept { return failed_; }

    bool require_eof() {
        if (remaining() != 0U) {
            return fail("cache", "trailing bytes after PropertyReferences");
        }
        return true;
    }

    bool raw(void* destination, const std::size_t count, const char* field) {
        if (count > remaining()) {
            return fail(field, "unexpected end of cache");
        }
        if (count != 0U) {
            std::memcpy(destination, bytes_ + position_, count);
        }
        position_ += count;
        return true;
    }

    bool u32(std::uint32_t& value, const char* field) {
        std::array<std::uint8_t, 4U> bytes{};
        if (!raw(bytes.data(), bytes.size(), field)) {
            return false;
        }
        value = static_cast<std::uint32_t>(bytes[0]) |
            (static_cast<std::uint32_t>(bytes[1]) << 8U) |
            (static_cast<std::uint32_t>(bytes[2]) << 16U) |
            (static_cast<std::uint32_t>(bytes[3]) << 24U);
        return true;
    }

    bool i32(std::int32_t& value, const char* field) {
        std::uint32_t unsigned_value = 0U;
        if (!u32(unsigned_value, field)) {
            return false;
        }
        std::memcpy(&value, &unsigned_value, sizeof(value));
        return true;
    }

    bool u64(std::uint64_t& value, const char* field) {
        std::array<std::uint8_t, 8U> bytes{};
        if (!raw(bytes.data(), bytes.size(), field)) {
            return false;
        }
        value = 0U;
        for (std::size_t index = 0U; index < bytes.size(); ++index) {
            value |= static_cast<std::uint64_t>(bytes[index]) << (index * 8U);
        }
        return true;
    }

    bool i64(std::int64_t& value, const char* field) {
        std::uint64_t unsigned_value = 0U;
        if (!u64(unsigned_value, field)) {
            return false;
        }
        std::memcpy(&value, &unsigned_value, sizeof(value));
        return true;
    }

    bool boolean(bool& value, const char* field) {
        std::int32_t encoded = 0;
        if (!i32(encoded, field)) {
            return false;
        }
        if (encoded != 0 && encoded != 1) {
            return fail(field, "archive bool is not canonical 0 or 1");
        }
        value = encoded == 1;
        return true;
    }

    bool count(std::size_t& value, const std::size_t minimum_bytes, const char* field) {
        std::int32_t encoded = 0;
        if (!i32(encoded, field)) {
            return false;
        }
        if (encoded < 0 || static_cast<std::size_t>(encoded) > kMaxContainerElements) {
            return fail(field, "container count is outside the supported bounds");
        }
        value = static_cast<std::size_t>(encoded);
        if (minimum_bytes != 0U &&
            value > remaining() / minimum_bytes) {
            return fail(field, "container count is not backed by the remaining bytes");
        }
        return true;
    }

    bool sia(archive_string& value, const char* field) {
        const std::size_t start = position_;
        std::int32_t length = 0;
        if (!i32(length, field)) {
            return false;
        }
        if (length == 0) {
            value.bytes.clear();
            return true;
        }
        if (length < 0 || static_cast<std::size_t>(length) > kMaxStringUnits) {
            position_ = start;
            return fail(field, "FStringInArchive length is invalid");
        }
        const std::size_t payload_size = static_cast<std::size_t>(length);
        if (payload_size + 1U > remaining()) {
            return fail(field, "FStringInArchive extends past end of cache");
        }
        const std::uint8_t* const payload = bytes_ + position_;
        if (payload[payload_size] != 0U ||
            std::find(payload, payload + payload_size, std::uint8_t{0U}) !=
                payload + payload_size) {
            return fail(field, "FStringInArchive has a missing or embedded NUL");
        }
        value.bytes.assign(reinterpret_cast<const char*>(payload), payload_size);
        position_ += payload_size + 1U;
        return true;
    }

    bool fstring(map_string& value, const char* field) {
        const std::size_t start = position_;
        std::int32_t encoded_length = 0;
        if (!i32(encoded_length, field)) {
            return false;
        }
        if (encoded_length == 0) {
            value.utf16 = false;
            value.payload.clear();
            return true;
        }

        const std::int64_t signed_length = encoded_length;
        const std::uint64_t total_units = signed_length < 0
            ? static_cast<std::uint64_t>(-signed_length)
            : static_cast<std::uint64_t>(signed_length);
        if (total_units <= 1U || total_units > kMaxStringUnits + 1U) {
            position_ = start;
            return fail(field, "FString length is non-canonical or outside bounds");
        }

        value.utf16 = signed_length < 0;
        const std::size_t unit_bytes = value.utf16 ? 2U : 1U;
        const std::size_t payload_size = static_cast<std::size_t>(total_units - 1U) * unit_bytes;
        const std::size_t encoded_size = payload_size + unit_bytes;
        if (encoded_size > remaining()) {
            return fail(field, "FString extends past end of cache");
        }
        const std::uint8_t* const payload = bytes_ + position_;
        if (value.utf16) {
            if (payload[payload_size] != 0U || payload[payload_size + 1U] != 0U ||
                !valid_utf16_payload(payload, payload_size)) {
                return fail(field, "UTF-16 FString is invalid or not NUL terminated");
            }
        } else if (payload[payload_size] != 0U ||
                   std::find(payload, payload + payload_size, std::uint8_t{0U}) !=
                       payload + payload_size) {
            return fail(field, "ANSI FString has a missing or embedded NUL");
        }
        value.payload.assign(payload, payload + payload_size);
        position_ += encoded_size;
        return true;
    }

    template <typename Value, typename Decode>
    bool array(
        std::vector<Value>& values,
        const std::size_t minimum_bytes,
        const char* field,
        Decode&& decode) {
        std::size_t element_count = 0U;
        if (!count(element_count, minimum_bytes, field)) {
            return false;
        }
        std::vector<Value> decoded;
        decoded.reserve(element_count);
        for (std::size_t index = 0U; index < element_count; ++index) {
            Value value{};
            if (!decode(*this, value)) {
                return false;
            }
            decoded.push_back(std::move(value));
        }
        values = std::move(decoded);
        return true;
    }

    bool fail(const char* field, const char* detail) {
        if (!failed_) {
            failed_ = true;
            error_.offset = position_;
            error_.field = field;
            error_.detail = detail;
        }
        return false;
    }

private:
    const std::uint8_t* bytes_;
    std::size_t size_;
    std::size_t position_ = 0U;
    codec_error& error_;
    bool failed_ = false;
};

class writer final {
public:
    explicit writer(codec_error& error) noexcept : error_(error) {}

    [[nodiscard]] bool failed() const noexcept { return failed_; }

    bool raw(const void* source, const std::size_t count, const char* field) {
        if (count > kMaxCacheBytes - bytes_.size()) {
            return fail(field, "encoded cache exceeds the byte limit");
        }
        if (count == 0U) {
            return true;
        }
        const auto* first = static_cast<const std::uint8_t*>(source);
        bytes_.insert(bytes_.end(), first, first + count);
        return true;
    }

    bool u32(const std::uint32_t value, const char* field) {
        const std::array<std::uint8_t, 4U> bytes{
            static_cast<std::uint8_t>(value),
            static_cast<std::uint8_t>(value >> 8U),
            static_cast<std::uint8_t>(value >> 16U),
            static_cast<std::uint8_t>(value >> 24U),
        };
        return raw(bytes.data(), bytes.size(), field);
    }

    bool i32(const std::int32_t value, const char* field) {
        std::uint32_t unsigned_value = 0U;
        std::memcpy(&unsigned_value, &value, sizeof(value));
        return u32(unsigned_value, field);
    }

    bool u64(const std::uint64_t value, const char* field) {
        std::array<std::uint8_t, 8U> bytes{};
        for (std::size_t index = 0U; index < bytes.size(); ++index) {
            bytes[index] = static_cast<std::uint8_t>(value >> (index * 8U));
        }
        return raw(bytes.data(), bytes.size(), field);
    }

    bool i64(const std::int64_t value, const char* field) {
        std::uint64_t unsigned_value = 0U;
        std::memcpy(&unsigned_value, &value, sizeof(value));
        return u64(unsigned_value, field);
    }

    bool boolean(const bool value, const char* field) {
        return i32(value ? 1 : 0, field);
    }

    bool count(const std::size_t value, const char* field) {
        if (value > kMaxContainerElements ||
            value > static_cast<std::size_t>((std::numeric_limits<std::int32_t>::max)())) {
            return fail(field, "container count is outside the supported bounds");
        }
        return i32(static_cast<std::int32_t>(value), field);
    }

    bool sia(const archive_string& value, const char* field) {
        if (value.bytes.empty()) {
            return i32(0, field);
        }
        if (value.bytes.size() > kMaxStringUnits ||
            value.bytes.find('\0') != std::string::npos) {
            return fail(field, "FStringInArchive payload is invalid");
        }
        return i32(static_cast<std::int32_t>(value.bytes.size()), field) &&
            raw(value.bytes.data(), value.bytes.size(), field) &&
            byte(0U, field);
    }

    bool fstring(const map_string& value, const char* field) {
        if (value.payload.empty()) {
            if (value.utf16) {
                return fail(field, "empty FString must use the canonical zero encoding");
            }
            return i32(0, field);
        }
        if (value.utf16) {
            if (!valid_utf16_payload(value.payload.data(), value.payload.size())) {
                return fail(field, "UTF-16 FString payload is invalid");
            }
            const std::size_t units = value.payload.size() / 2U;
            if (units > kMaxStringUnits) {
                return fail(field, "UTF-16 FString exceeds the unit limit");
            }
            return i32(-static_cast<std::int32_t>(units + 1U), field) &&
                raw(value.payload.data(), value.payload.size(), field) &&
                byte(0U, field) && byte(0U, field);
        }
        if (value.payload.size() > kMaxStringUnits ||
            std::find(value.payload.begin(), value.payload.end(), std::uint8_t{0U}) !=
                value.payload.end()) {
            return fail(field, "ANSI FString payload is invalid");
        }
        return i32(static_cast<std::int32_t>(value.payload.size() + 1U), field) &&
            raw(value.payload.data(), value.payload.size(), field) && byte(0U, field);
    }

    template <typename Value, typename Encode>
    bool array(const std::vector<Value>& values, const char* field, Encode&& encode) {
        if (!count(values.size(), field)) {
            return false;
        }
        for (const Value& value : values) {
            if (!encode(*this, value)) {
                return false;
            }
        }
        return true;
    }

    std::vector<std::uint8_t> finish() { return std::move(bytes_); }

    bool fail(const char* field, const char* detail) {
        if (!failed_) {
            failed_ = true;
            error_.offset = bytes_.size();
            error_.field = field;
            error_.detail = detail;
        }
        return false;
    }

private:
    bool byte(const std::uint8_t value, const char* field) {
        return raw(&value, sizeof(value), field);
    }

    std::vector<std::uint8_t> bytes_;
    codec_error& error_;
    bool failed_ = false;
};

bool read_data_type(reader& input, data_type& value) {
    return input.boolean(value.is_reference, "DataType.bIsReference") &&
        input.boolean(value.is_object_const, "DataType.bIsObjectConst") &&
        input.boolean(value.is_object_handle, "DataType.bIsObjectHandle") &&
        input.boolean(value.is_const_handle, "DataType.bIsConstHandle") &&
        input.boolean(value.is_auto, "DataType.bIsAuto") &&
        input.boolean(value.if_handle_then_const, "DataType.bIfHandleThenConst") &&
        input.i64(value.type_info, "DataType.TypeInfo") &&
        input.i32(value.token_type, "DataType.TokenType");
}

bool write_data_type(writer& output, const data_type& value) {
    return output.boolean(value.is_reference, "DataType.bIsReference") &&
        output.boolean(value.is_object_const, "DataType.bIsObjectConst") &&
        output.boolean(value.is_object_handle, "DataType.bIsObjectHandle") &&
        output.boolean(value.is_const_handle, "DataType.bIsConstHandle") &&
        output.boolean(value.is_auto, "DataType.bIsAuto") &&
        output.boolean(value.if_handle_then_const, "DataType.bIfHandleThenConst") &&
        output.i64(value.type_info, "DataType.TypeInfo") &&
        output.i32(value.token_type, "DataType.TokenType");
}

bool read_i32(reader& input, std::int32_t& value) { return input.i32(value, "int32"); }
bool write_i32(writer& output, const std::int32_t value) { return output.i32(value, "int32"); }
bool read_i64(reader& input, std::int64_t& value) { return input.i64(value, "int64"); }
bool write_i64(writer& output, const std::int64_t value) { return output.i64(value, "int64"); }
bool read_sia(reader& input, archive_string& value) {
    return input.sia(value, "FStringInArchive");
}
bool write_sia(writer& output, const archive_string& value) {
    return output.sia(value, "FStringInArchive");
}

bool read_function_signature(reader& input, function_signature& value) {
    return input.sia(value.name, "FunctionSignature.Name") &&
        input.sia(value.name_space, "FunctionSignature.Namespace") &&
        input.array(
            value.parameter_types, kDataTypeBytes, "FunctionSignature.ParameterTypes",
            read_data_type) &&
        input.array(value.parameter_flags, 4U, "FunctionSignature.ParameterFlags", read_i32) &&
        input.array(
            value.parameter_default_args, 4U, "FunctionSignature.ParameterDefaultArgs",
            read_sia) &&
        read_data_type(input, value.return_type);
}

bool write_function_signature(writer& output, const function_signature& value) {
    return output.sia(value.name, "FunctionSignature.Name") &&
        output.sia(value.name_space, "FunctionSignature.Namespace") &&
        output.array(value.parameter_types, "FunctionSignature.ParameterTypes", write_data_type) &&
        output.array(value.parameter_flags, "FunctionSignature.ParameterFlags", write_i32) &&
        output.array(
            value.parameter_default_args, "FunctionSignature.ParameterDefaultArgs", write_sia) &&
        write_data_type(output, value.return_type);
}

bool read_precompiled_function(reader& input, precompiled_function& value);
bool write_precompiled_function(writer& output, const precompiled_function& value);

bool read_precompiled_function(reader& input, precompiled_function& value) {
    if (!input.sia(value.function_name, "Function.FunctionName") ||
        !input.sia(value.name_space, "Function.Namespace") ||
        !read_data_type(input, value.return_type) ||
        !input.array(
            value.parameter_types, kDataTypeBytes, "Function.ParameterTypes", read_data_type) ||
        !input.array(value.parameter_names, 4U, "Function.ParameterNames", read_sia) ||
        !input.array(value.parameter_flags, 4U, "Function.ParameterFlags", read_i32) ||
        !input.array(
            value.parameter_default_args, 4U, "Function.ParameterDefaultArgs", read_sia) ||
        !input.i32(value.function_traits, "Function.FunctionTraits") ||
        !input.array(value.byte_code, 4U, "Function.ByteCode", read_i32) ||
        !input.array(
            value.byte_code_references, 4U, "Function.ByteCodeReferences", read_i32) ||
        !input.i32(value.variable_space, "Function.VariableSpace") ||
        !input.array(
            value.object_variable_types, 8U, "Function.ObjVariableTypes", read_i64) ||
        !input.array(
            value.object_variable_positions, 4U, "Function.ObjVariablePos", read_i32) ||
        !input.i32(value.object_variables_on_heap, "Function.ObjVariablesOnHeap") ||
        !input.array(
            value.variable_info_program_positions, 4U, "Function.VarInfoProgramPos", read_i32) ||
        !input.array(
            value.variable_info_offsets, 4U, "Function.VarInfoOffset", read_i32) ||
        !input.array(
            value.variable_info_options, 4U, "Function.VarInfoOption", read_i32) ||
        !input.i32(value.stack_needed, "Function.StackNeeded") ||
        !input.u32(value.id, "Function.Id") ||
        !input.i32(value.declared_at, "Function.DeclaredAt") ||
        !input.array(value.line_numbers, 4U, "Function.LineNumbers", read_i32) ||
        !input.boolean(value.is_unreal_function, "Function.bIsUFunction")) {
        return false;
    }
    if (!value.is_unreal_function) {
        return true;
    }
    return input.sia(value.unreal_function_name, "Function.UnrealFunctionName") &&
        input.array(value.metadata_specifiers, 4U, "Function.MetaSpec", read_sia) &&
        input.array(value.metadata_values, 4U, "Function.MetaValues", read_sia) &&
        input.boolean(value.blueprint_callable, "Function.bBlueprintCallable") &&
        input.boolean(value.blueprint_override, "Function.bBlueprintOverride") &&
        input.boolean(value.blueprint_event, "Function.bBlueprintEvent") &&
        input.boolean(value.blueprint_pure, "Function.bBlueprintPure") &&
        input.boolean(value.net_function, "Function.bNetFunction") &&
        input.boolean(value.net_multicast, "Function.bNetMulticast") &&
        input.boolean(value.net_client, "Function.bNetClient") &&
        input.boolean(value.net_server, "Function.bNetServer") &&
        input.boolean(value.net_validate, "Function.bNetValidate") &&
        input.boolean(value.unreliable, "Function.bUnreliable") &&
        input.boolean(value.blueprint_authority_only, "Function.bBlueprintAuthorityOnly") &&
        input.boolean(value.exec, "Function.bExec") &&
        input.boolean(value.can_override_event, "Function.bCanOverrideEvent") &&
        input.boolean(value.dev_function, "Function.bDevFunction") &&
        input.boolean(value.is_static, "Function.bIsStatic") &&
        input.boolean(value.is_const_method, "Function.bIsConstMethod") &&
        input.boolean(value.thread_safe, "Function.bThreadSafe") &&
        input.boolean(value.is_no_op, "Function.bIsNoOp");
}

bool write_precompiled_function(writer& output, const precompiled_function& value) {
    if (!output.sia(value.function_name, "Function.FunctionName") ||
        !output.sia(value.name_space, "Function.Namespace") ||
        !write_data_type(output, value.return_type) ||
        !output.array(value.parameter_types, "Function.ParameterTypes", write_data_type) ||
        !output.array(value.parameter_names, "Function.ParameterNames", write_sia) ||
        !output.array(value.parameter_flags, "Function.ParameterFlags", write_i32) ||
        !output.array(
            value.parameter_default_args, "Function.ParameterDefaultArgs", write_sia) ||
        !output.i32(value.function_traits, "Function.FunctionTraits") ||
        !output.array(value.byte_code, "Function.ByteCode", write_i32) ||
        !output.array(
            value.byte_code_references, "Function.ByteCodeReferences", write_i32) ||
        !output.i32(value.variable_space, "Function.VariableSpace") ||
        !output.array(value.object_variable_types, "Function.ObjVariableTypes", write_i64) ||
        !output.array(value.object_variable_positions, "Function.ObjVariablePos", write_i32) ||
        !output.i32(value.object_variables_on_heap, "Function.ObjVariablesOnHeap") ||
        !output.array(
            value.variable_info_program_positions, "Function.VarInfoProgramPos", write_i32) ||
        !output.array(value.variable_info_offsets, "Function.VarInfoOffset", write_i32) ||
        !output.array(value.variable_info_options, "Function.VarInfoOption", write_i32) ||
        !output.i32(value.stack_needed, "Function.StackNeeded") ||
        !output.u32(value.id, "Function.Id") ||
        !output.i32(value.declared_at, "Function.DeclaredAt") ||
        !output.array(value.line_numbers, "Function.LineNumbers", write_i32) ||
        !output.boolean(value.is_unreal_function, "Function.bIsUFunction")) {
        return false;
    }
    if (!value.is_unreal_function) {
        return true;
    }
    return output.sia(value.unreal_function_name, "Function.UnrealFunctionName") &&
        output.array(value.metadata_specifiers, "Function.MetaSpec", write_sia) &&
        output.array(value.metadata_values, "Function.MetaValues", write_sia) &&
        output.boolean(value.blueprint_callable, "Function.bBlueprintCallable") &&
        output.boolean(value.blueprint_override, "Function.bBlueprintOverride") &&
        output.boolean(value.blueprint_event, "Function.bBlueprintEvent") &&
        output.boolean(value.blueprint_pure, "Function.bBlueprintPure") &&
        output.boolean(value.net_function, "Function.bNetFunction") &&
        output.boolean(value.net_multicast, "Function.bNetMulticast") &&
        output.boolean(value.net_client, "Function.bNetClient") &&
        output.boolean(value.net_server, "Function.bNetServer") &&
        output.boolean(value.net_validate, "Function.bNetValidate") &&
        output.boolean(value.unreliable, "Function.bUnreliable") &&
        output.boolean(value.blueprint_authority_only, "Function.bBlueprintAuthorityOnly") &&
        output.boolean(value.exec, "Function.bExec") &&
        output.boolean(value.can_override_event, "Function.bCanOverrideEvent") &&
        output.boolean(value.dev_function, "Function.bDevFunction") &&
        output.boolean(value.is_static, "Function.bIsStatic") &&
        output.boolean(value.is_const_method, "Function.bIsConstMethod") &&
        output.boolean(value.thread_safe, "Function.bThreadSafe") &&
        output.boolean(value.is_no_op, "Function.bIsNoOp");
}

bool read_precompiled_property(reader& input, precompiled_property& value) {
    if (!input.sia(value.name, "Property.Name") ||
        !read_data_type(input, value.type) ||
        !input.boolean(value.is_private, "Property.bIsPrivate") ||
        !input.boolean(value.is_protected, "Property.bIsProtected") ||
        !input.boolean(value.is_unreal_property, "Property.bIsUnrealProperty")) {
        return false;
    }
    if (!value.is_unreal_property) {
        return true;
    }
    if (!input.array(value.metadata_specifiers, 4U, "Property.MetaSpec", read_sia) ||
        !input.array(value.metadata_values, 4U, "Property.MetaValues", read_sia) ||
        !input.boolean(value.blueprint_readable, "Property.bBlueprintReadable") ||
        !input.boolean(value.blueprint_writable, "Property.bBlueprintWritable") ||
        !input.boolean(value.edit_const, "Property.bEditConst") ||
        !input.boolean(value.editable_on_defaults, "Property.bEditableOnDefaults") ||
        !input.boolean(value.editable_on_instance, "Property.bEditableOnInstance") ||
        !input.boolean(value.instanced_reference, "Property.bInstancedReference") ||
        !input.boolean(value.persistent_instance, "Property.bPersistentInstance") ||
        !input.boolean(value.advanced_display, "Property.bAdvancedDisplay") ||
        !input.boolean(value.transient, "Property.bTransient") ||
        !input.boolean(value.replicated, "Property.bReplicated") ||
        !input.boolean(value.skip_replication, "Property.bSkipReplication") ||
        !input.boolean(value.skip_serialization, "Property.bSkipSerialization") ||
        !input.boolean(value.save_game, "Property.bSaveGame")) {
        return false;
    }
    if (value.replicated &&
        (!input.i32(value.replication_condition, "Property.ReplicationCondition") ||
         !input.boolean(value.rep_notify, "Property.bRepNotify"))) {
        return false;
    }
    return input.boolean(value.config, "Property.bConfig") &&
        input.boolean(value.interp, "Property.bInterp") &&
        input.boolean(value.asset_registry_searchable, "Property.bAssetRegistrySearchable");
}

bool write_precompiled_property(writer& output, const precompiled_property& value) {
    if (!output.sia(value.name, "Property.Name") ||
        !write_data_type(output, value.type) ||
        !output.boolean(value.is_private, "Property.bIsPrivate") ||
        !output.boolean(value.is_protected, "Property.bIsProtected") ||
        !output.boolean(value.is_unreal_property, "Property.bIsUnrealProperty")) {
        return false;
    }
    if (!value.is_unreal_property) {
        return true;
    }
    if (!output.array(value.metadata_specifiers, "Property.MetaSpec", write_sia) ||
        !output.array(value.metadata_values, "Property.MetaValues", write_sia) ||
        !output.boolean(value.blueprint_readable, "Property.bBlueprintReadable") ||
        !output.boolean(value.blueprint_writable, "Property.bBlueprintWritable") ||
        !output.boolean(value.edit_const, "Property.bEditConst") ||
        !output.boolean(value.editable_on_defaults, "Property.bEditableOnDefaults") ||
        !output.boolean(value.editable_on_instance, "Property.bEditableOnInstance") ||
        !output.boolean(value.instanced_reference, "Property.bInstancedReference") ||
        !output.boolean(value.persistent_instance, "Property.bPersistentInstance") ||
        !output.boolean(value.advanced_display, "Property.bAdvancedDisplay") ||
        !output.boolean(value.transient, "Property.bTransient") ||
        !output.boolean(value.replicated, "Property.bReplicated") ||
        !output.boolean(value.skip_replication, "Property.bSkipReplication") ||
        !output.boolean(value.skip_serialization, "Property.bSkipSerialization") ||
        !output.boolean(value.save_game, "Property.bSaveGame")) {
        return false;
    }
    if (value.replicated &&
        (!output.i32(value.replication_condition, "Property.ReplicationCondition") ||
         !output.boolean(value.rep_notify, "Property.bRepNotify"))) {
        return false;
    }
    return output.boolean(value.config, "Property.bConfig") &&
        output.boolean(value.interp, "Property.bInterp") &&
        output.boolean(value.asset_registry_searchable, "Property.bAssetRegistrySearchable");
}

bool read_precompiled_class(reader& input, precompiled_class& value) {
    if (!input.sia(value.class_name, "Class.ClassName") ||
        !input.sia(value.name_space, "Class.Namespace") ||
        !input.i32(value.flags, "Class.Flags") ||
        !input.array(
            value.properties, kMinimumPropertyBytes, "Class.Properties",
            read_precompiled_property) ||
        !input.array(
            value.methods, kMinimumFunctionBytes, "Class.Methods", read_precompiled_function) ||
        !input.array(value.method_table, 4U, "Class.MethodTable", read_i32) ||
        !input.i64(value.derived_from, "Class.DerivedFrom") ||
        !input.i64(value.shadow_type, "Class.ShadowType") ||
        !input.array(
            value.constructors, kMinimumFunctionBytes, "Class.Constructors",
            read_precompiled_function) ||
        !input.array(value.factory_references, 8U, "Class.FactoryRefs", read_i64) ||
        !input.array(value.behaviour_references, 8U, "Class.BehaviorRefs", read_i64) ||
        !input.array(
            value.behaviour_functions, kMinimumFunctionBytes, "Class.BehaviorFunctions",
            read_precompiled_function) ||
        !input.array(
            value.behaviour_function_types, 4U, "Class.BehaviorFunctionTypes", read_i32) ||
        !input.boolean(value.is_in_preprocessor, "Class.bIsInPreprocessor")) {
        return false;
    }
    if (!value.is_in_preprocessor) {
        return true;
    }
    return input.sia(value.super_class, "Class.SuperClass") &&
        input.sia(value.code_super_class, "Class.CodeSuperClass") &&
        input.boolean(value.super_is_code_class, "Class.bSuperIsCodeClass") &&
        input.boolean(value.abstract, "Class.bAbstract") &&
        input.boolean(value.transient, "Class.bTransient") &&
        input.boolean(value.hide_dropdown, "Class.bHideDropdown") &&
        input.boolean(value.default_to_instanced, "Class.bDefaultToInstanced") &&
        input.boolean(value.edit_inline_new, "Class.bEditInlineNew") &&
        input.boolean(value.is_deprecated_class, "Class.bIsDeprecatedClass") &&
        input.sia(value.config_name, "Class.ConfigName") &&
        input.sia(
            value.static_class_global_variable_name, "Class.StaticClassGlobalVariableName") &&
        input.boolean(value.placeable, "Class.bPlaceable") &&
        input.array(value.metadata_specifiers, 4U, "Class.MetaSpec", read_sia) &&
        input.array(value.metadata_values, 4U, "Class.MetaValues", read_sia) &&
        input.sia(value.compose_onto_class_name, "Class.ComposeOntoClassName");
}

bool write_precompiled_class(writer& output, const precompiled_class& value) {
    if (!output.sia(value.class_name, "Class.ClassName") ||
        !output.sia(value.name_space, "Class.Namespace") ||
        !output.i32(value.flags, "Class.Flags") ||
        !output.array(value.properties, "Class.Properties", write_precompiled_property) ||
        !output.array(value.methods, "Class.Methods", write_precompiled_function) ||
        !output.array(value.method_table, "Class.MethodTable", write_i32) ||
        !output.i64(value.derived_from, "Class.DerivedFrom") ||
        !output.i64(value.shadow_type, "Class.ShadowType") ||
        !output.array(value.constructors, "Class.Constructors", write_precompiled_function) ||
        !output.array(value.factory_references, "Class.FactoryRefs", write_i64) ||
        !output.array(value.behaviour_references, "Class.BehaviorRefs", write_i64) ||
        !output.array(
            value.behaviour_functions, "Class.BehaviorFunctions", write_precompiled_function) ||
        !output.array(
            value.behaviour_function_types, "Class.BehaviorFunctionTypes", write_i32) ||
        !output.boolean(value.is_in_preprocessor, "Class.bIsInPreprocessor")) {
        return false;
    }
    if (!value.is_in_preprocessor) {
        return true;
    }
    return output.sia(value.super_class, "Class.SuperClass") &&
        output.sia(value.code_super_class, "Class.CodeSuperClass") &&
        output.boolean(value.super_is_code_class, "Class.bSuperIsCodeClass") &&
        output.boolean(value.abstract, "Class.bAbstract") &&
        output.boolean(value.transient, "Class.bTransient") &&
        output.boolean(value.hide_dropdown, "Class.bHideDropdown") &&
        output.boolean(value.default_to_instanced, "Class.bDefaultToInstanced") &&
        output.boolean(value.edit_inline_new, "Class.bEditInlineNew") &&
        output.boolean(value.is_deprecated_class, "Class.bIsDeprecatedClass") &&
        output.sia(value.config_name, "Class.ConfigName") &&
        output.sia(
            value.static_class_global_variable_name, "Class.StaticClassGlobalVariableName") &&
        output.boolean(value.placeable, "Class.bPlaceable") &&
        output.array(value.metadata_specifiers, "Class.MetaSpec", write_sia) &&
        output.array(value.metadata_values, "Class.MetaValues", write_sia) &&
        output.sia(value.compose_onto_class_name, "Class.ComposeOntoClassName");
}

bool read_precompiled_enum(reader& input, precompiled_enum& value) {
    return input.sia(value.name, "Enum.Name") &&
        input.sia(value.name_space, "Enum.Namespace") &&
        input.array(value.names, 4U, "Enum.EnumNames", read_sia) &&
        input.array(value.values, 4U, "Enum.EnumValues", read_i32);
}

bool write_precompiled_enum(writer& output, const precompiled_enum& value) {
    return output.sia(value.name, "Enum.Name") &&
        output.sia(value.name_space, "Enum.Namespace") &&
        output.array(value.names, "Enum.EnumNames", write_sia) &&
        output.array(value.values, "Enum.EnumValues", write_i32);
}

bool read_precompiled_global(reader& input, precompiled_global& value) {
    if (!input.sia(value.name, "Global.Name") ||
        !input.sia(value.name_space, "Global.Namespace") ||
        !read_data_type(input, value.type) ||
        !input.boolean(value.is_default_init, "Global.bIsDefaultInit")) {
        return false;
    }
    if (value.is_default_init) {
        return true;
    }
    if (!input.boolean(value.is_pure_constant, "Global.bIsPureConstant")) {
        return false;
    }
    if (value.is_pure_constant) {
        return input.u64(value.pure_constant_value, "Global.PureConstantValue");
    }
    // The fork serializes InitFunc even when bHasInitFunction is false.
    return input.boolean(value.has_init_function, "Global.bHasInitFunction") &&
        read_precompiled_function(input, value.init_function);
}

bool write_precompiled_global(writer& output, const precompiled_global& value) {
    if (!output.sia(value.name, "Global.Name") ||
        !output.sia(value.name_space, "Global.Namespace") ||
        !write_data_type(output, value.type) ||
        !output.boolean(value.is_default_init, "Global.bIsDefaultInit")) {
        return false;
    }
    if (value.is_default_init) {
        return true;
    }
    if (!output.boolean(value.is_pure_constant, "Global.bIsPureConstant")) {
        return false;
    }
    if (value.is_pure_constant) {
        return output.u64(value.pure_constant_value, "Global.PureConstantValue");
    }
    return output.boolean(value.has_init_function, "Global.bHasInitFunction") &&
        write_precompiled_function(output, value.init_function);
}

bool read_function_import(reader& input, function_import& value) {
    return input.sia(value.imported_from_module, "FunctionImport.ImportedFromModule") &&
        read_function_signature(input, value.signature);
}

bool write_function_import(writer& output, const function_import& value) {
    return output.sia(value.imported_from_module, "FunctionImport.ImportedFromModule") &&
        write_function_signature(output, value.signature);
}

bool read_precompiled_module(reader& input, precompiled_module& value) {
    return input.sia(value.module_name, "Module.ModuleName") &&
        input.array(
            value.functions, kMinimumFunctionBytes, "Module.Functions",
            read_precompiled_function) &&
        input.array(value.classes, kMinimumClassBytes, "Module.Classes", read_precompiled_class) &&
        input.array(value.enums, kMinimumEnumBytes, "Module.Enums", read_precompiled_enum) &&
        input.array(
            value.global_variables, kMinimumGlobalBytes, "Module.GlobalVariables",
            read_precompiled_global) &&
        input.array(
            value.function_imports, kMinimumImportBytes, "Module.FunctionImports",
            read_function_import) &&
        input.i64(value.code_hash, "Module.CodeHash") &&
        input.array(value.imported_modules, 4U, "Module.ImportedModules", read_sia) &&
        input.sia(value.statics_class_name, "Module.StaticsClassName") &&
        input.array(value.declared_events, 4U, "Module.DeclaredEvents", read_sia) &&
        input.array(value.declared_delegates, 4U, "Module.DeclaredDelegates", read_sia) &&
        input.sia(value.script_relative_filename, "Module.ScriptRelativeFilename") &&
        input.array(value.post_init_functions, 4U, "Module.PostInitFunctions", read_sia);
}

bool write_precompiled_module(writer& output, const precompiled_module& value) {
    return output.sia(value.module_name, "Module.ModuleName") &&
        output.array(value.functions, "Module.Functions", write_precompiled_function) &&
        output.array(value.classes, "Module.Classes", write_precompiled_class) &&
        output.array(value.enums, "Module.Enums", write_precompiled_enum) &&
        output.array(value.global_variables, "Module.GlobalVariables", write_precompiled_global) &&
        output.array(value.function_imports, "Module.FunctionImports", write_function_import) &&
        output.i64(value.code_hash, "Module.CodeHash") &&
        output.array(value.imported_modules, "Module.ImportedModules", write_sia) &&
        output.sia(value.statics_class_name, "Module.StaticsClassName") &&
        output.array(value.declared_events, "Module.DeclaredEvents", write_sia) &&
        output.array(value.declared_delegates, "Module.DeclaredDelegates", write_sia) &&
        output.sia(value.script_relative_filename, "Module.ScriptRelativeFilename") &&
        output.array(value.post_init_functions, "Module.PostInitFunctions", write_sia);
}

bool read_type_reference(reader& input, type_reference& value) {
    return input.sia(value.name, "TypeReference.Name") &&
        input.sia(value.module, "TypeReference.Module") &&
        input.sia(value.name_space, "TypeReference.Namespace") &&
        input.array(value.sub_types, kDataTypeBytes, "TypeReference.SubTypes", read_data_type);
}

bool write_type_reference(writer& output, const type_reference& value) {
    return output.sia(value.name, "TypeReference.Name") &&
        output.sia(value.module, "TypeReference.Module") &&
        output.sia(value.name_space, "TypeReference.Namespace") &&
        output.array(value.sub_types, "TypeReference.SubTypes", write_data_type);
}

bool read_function_reference(reader& input, function_reference& value) {
    return input.sia(value.name, "FunctionReference.Name") &&
        input.sia(value.module, "FunctionReference.Module") &&
        input.sia(value.name_space, "FunctionReference.Namespace") &&
        input.boolean(value.is_const, "FunctionReference.bIsConst") &&
        input.boolean(value.is_imported_decl, "FunctionReference.bIsImportedDecl") &&
        input.boolean(value.is_method, "FunctionReference.bIsMethod") &&
        input.i64(value.object_type, "FunctionReference.ObjectType") &&
        input.array(
            value.parameter_types, kDataTypeBytes, "FunctionReference.ParameterTypes",
            read_data_type) &&
        read_data_type(input, value.return_type);
}

bool write_function_reference(writer& output, const function_reference& value) {
    return output.sia(value.name, "FunctionReference.Name") &&
        output.sia(value.module, "FunctionReference.Module") &&
        output.sia(value.name_space, "FunctionReference.Namespace") &&
        output.boolean(value.is_const, "FunctionReference.bIsConst") &&
        output.boolean(value.is_imported_decl, "FunctionReference.bIsImportedDecl") &&
        output.boolean(value.is_method, "FunctionReference.bIsMethod") &&
        output.i64(value.object_type, "FunctionReference.ObjectType") &&
        output.array(
            value.parameter_types, "FunctionReference.ParameterTypes", write_data_type) &&
        write_data_type(output, value.return_type);
}

bool read_global_reference(reader& input, global_reference& value) {
    if (!input.sia(value.name, "GlobalReference.Name") ||
        !input.sia(value.module, "GlobalReference.Module") ||
        !input.sia(value.name_space, "GlobalReference.Namespace") ||
        !input.boolean(value.is_string, "GlobalReference.bIsString")) {
        return false;
    }
    if (value.is_string && !valid_utf8(value.name.bytes)) {
        return input.fail(
            "GlobalReference.Name", "script string literal is not canonical UTF-8");
    }
    return true;
}

bool write_global_reference(writer& output, const global_reference& value) {
    if (value.is_string && !valid_utf8(value.name.bytes)) {
        return output.fail(
            "GlobalReference.Name", "script string literal is not canonical UTF-8");
    }
    return output.sia(value.name, "GlobalReference.Name") &&
        output.sia(value.module, "GlobalReference.Module") &&
        output.sia(value.name_space, "GlobalReference.Namespace") &&
        output.boolean(value.is_string, "GlobalReference.bIsString");
}

bool read_property_reference(reader& input, property_reference& value) {
    return input.sia(value.name, "PropertyReference.Name") &&
        input.i32(value.old_type_id, "PropertyReference.OldTypeId");
}

bool write_property_reference(writer& output, const property_reference& value) {
    return output.sia(value.name, "PropertyReference.Name") &&
        output.i32(value.old_type_id, "PropertyReference.OldTypeId");
}

template <typename Key, typename Value, typename ReadKey, typename ReadValue>
bool read_map(
    reader& input,
    std::vector<std::pair<Key, Value>>& values,
    const std::size_t minimum_entry_bytes,
    const char* field,
    ReadKey&& read_key,
    ReadValue&& read_value) {
    std::size_t element_count = 0U;
    if (!input.count(element_count, minimum_entry_bytes, field)) {
        return false;
    }
    std::vector<std::pair<Key, Value>> decoded;
    decoded.reserve(element_count);
    for (std::size_t index = 0U; index < element_count; ++index) {
        Key key{};
        Value value{};
        if (!read_key(input, key) || !read_value(input, value)) {
            return false;
        }
        decoded.emplace_back(std::move(key), std::move(value));
    }
    values = std::move(decoded);
    return true;
}

template <typename Key, typename Value, typename WriteKey, typename WriteValue>
bool write_map(
    writer& output,
    const std::vector<std::pair<Key, Value>>& values,
    const char* field,
    WriteKey&& write_key,
    WriteValue&& write_value) {
    if (!output.count(values.size(), field)) {
        return false;
    }
    for (const auto& entry : values) {
        if (!write_key(output, entry.first) || !write_value(output, entry.second)) {
            return false;
        }
    }
    return true;
}

bool read_map_string(reader& input, map_string& value) {
    return input.fstring(value, "Modules.Key");
}

template <typename Key, typename Value>
bool require_unique_numeric_keys(
    reader& input,
    const std::vector<std::pair<Key, Value>>& values,
    const char* field) {
    std::unordered_set<Key> keys;
    keys.reserve(values.size());
    for (const auto& entry : values) {
        if (!keys.emplace(entry.first).second) {
            return input.fail(field, "serialized TMap contains a duplicate key");
        }
    }
    return true;
}

bool require_unique_module_keys(
    reader& input,
    const std::vector<std::pair<map_string, precompiled_module>>& values) {
    std::unordered_set<std::string> keys;
    keys.reserve(values.size());
    for (const auto& entry : values) {
        std::string key;
        key.reserve(entry.first.payload.size() + 1U);
        key.push_back(entry.first.utf16 ? 'u' : 'a');
        if (!entry.first.payload.empty()) {
            key.append(
                reinterpret_cast<const char*>(entry.first.payload.data()),
                entry.first.payload.size());
        }
        if (!keys.emplace(std::move(key)).second) {
            return input.fail("Modules", "serialized TMap contains a duplicate key");
        }
    }
    return true;
}

template <typename Key, typename Value>
bool require_unique_numeric_keys(
    writer& output,
    const std::vector<std::pair<Key, Value>>& values,
    const char* field) {
    std::unordered_set<Key> keys;
    keys.reserve(values.size());
    for (const auto& entry : values) {
        if (!keys.emplace(entry.first).second) {
            return output.fail(field, "serialized TMap contains a duplicate key");
        }
    }
    return true;
}

bool require_unique_module_keys(
    writer& output,
    const std::vector<std::pair<map_string, precompiled_module>>& values) {
    std::unordered_set<std::string> keys;
    keys.reserve(values.size());
    for (const auto& entry : values) {
        if (entry.first.payload.empty() && entry.first.utf16) {
            return output.fail("Modules", "empty FString key has a non-canonical encoding");
        }
        std::string key;
        key.reserve(entry.first.payload.size() + 1U);
        key.push_back(entry.first.utf16 ? 'u' : 'a');
        if (!entry.first.payload.empty()) {
            key.append(
                reinterpret_cast<const char*>(entry.first.payload.data()),
                entry.first.payload.size());
        }
        if (!keys.emplace(std::move(key)).second) {
            return output.fail("Modules", "serialized TMap contains a duplicate key");
        }
    }
    return true;
}
bool write_map_string(writer& output, const map_string& value) {
    return output.fstring(value, "Modules.Key");
}

bool read_cache(reader& input, cache& value) {
    if (!(input.raw(value.data_guid.data(), value.data_guid.size(), "DataGuid") &&
        input.i32(value.build_identifier, "BuildIdentifier") &&
        read_map(
            input, value.modules, kMinimumModulePairBytes, "Modules", read_map_string,
            read_precompiled_module) &&
        read_map(
            input, value.type_references, 24U, "TypeReferences", read_i64,
            read_type_reference) &&
        read_map(
            input, value.type_id_reference_to_pointer, 12U,
            "TypeIdReferenceToPointer", read_i32, read_i64) &&
        read_map(
            input, value.function_references, 80U, "FunctionReferences", read_i64,
            read_function_reference) &&
        read_map(
            input, value.function_id_reference_to_pointer, 12U,
            "FunctionIdReferenceToPointer", read_i32, read_i64) &&
        read_map(
            input, value.global_references, 24U, "GlobalReferences", read_i64,
            read_global_reference) &&
        input.array(value.static_names, 4U, "StaticNames", read_sia) &&
        read_map(
            input, value.property_references, 16U, "PropertyReferences", read_i64,
            read_property_reference) &&
        input.require_eof())) {
        return false;
    }
    return require_unique_module_keys(input, value.modules) &&
        require_unique_numeric_keys(input, value.type_references, "TypeReferences") &&
        require_unique_numeric_keys(
            input, value.type_id_reference_to_pointer, "TypeIdReferenceToPointer") &&
        require_unique_numeric_keys(input, value.function_references, "FunctionReferences") &&
        require_unique_numeric_keys(
            input, value.function_id_reference_to_pointer, "FunctionIdReferenceToPointer") &&
        require_unique_numeric_keys(input, value.global_references, "GlobalReferences") &&
        require_unique_numeric_keys(input, value.property_references, "PropertyReferences");
}

bool write_cache(writer& output, const cache& value) {
    if (!(require_unique_module_keys(output, value.modules) &&
          require_unique_numeric_keys(output, value.type_references, "TypeReferences") &&
          require_unique_numeric_keys(
              output, value.type_id_reference_to_pointer, "TypeIdReferenceToPointer") &&
          require_unique_numeric_keys(output, value.function_references, "FunctionReferences") &&
          require_unique_numeric_keys(
              output, value.function_id_reference_to_pointer,
              "FunctionIdReferenceToPointer") &&
          require_unique_numeric_keys(output, value.global_references, "GlobalReferences") &&
          require_unique_numeric_keys(
              output, value.property_references, "PropertyReferences"))) {
        return false;
    }
    return output.raw(value.data_guid.data(), value.data_guid.size(), "DataGuid") &&
        output.i32(value.build_identifier, "BuildIdentifier") &&
        write_map(
            output, value.modules, "Modules", write_map_string, write_precompiled_module) &&
        write_map(
            output, value.type_references, "TypeReferences", write_i64,
            write_type_reference) &&
        write_map(
            output, value.type_id_reference_to_pointer, "TypeIdReferenceToPointer", write_i32,
            write_i64) &&
        write_map(
            output, value.function_references, "FunctionReferences", write_i64,
            write_function_reference) &&
        write_map(
            output, value.function_id_reference_to_pointer, "FunctionIdReferenceToPointer",
            write_i32, write_i64) &&
        write_map(
            output, value.global_references, "GlobalReferences", write_i64,
            write_global_reference) &&
        output.array(value.static_names, "StaticNames", write_sia) &&
        write_map(
            output, value.property_references, "PropertyReferences", write_i64,
            write_property_reference);
}

} // namespace

bool decode(
    const std::uint8_t* const bytes,
    const std::size_t size,
    cache& output,
    codec_error& error) noexcept {
    error = {};
    if ((bytes == nullptr && size != 0U) || size > kMaxCacheBytes) {
        error.field = "cache";
        error.detail = "cache pointer or byte length is invalid";
        return false;
    }
    try {
        cache decoded;
        reader input(bytes, size, error);
        if (!read_cache(input, decoded)) {
            return false;
        }
        output = std::move(decoded);
        return true;
    } catch (const std::bad_alloc&) {
        error.field = "cache";
        error.detail = "allocation failed within the bounded decoder";
        return false;
    } catch (const std::exception& exception) {
        error.field = "cache";
        error.detail = exception.what();
        return false;
    } catch (...) {
        error.field = "cache";
        error.detail = "unexpected decoder failure";
        return false;
    }
}

bool encode(const cache& input, std::vector<std::uint8_t>& output, codec_error& error) noexcept {
    error = {};
    try {
        writer encoded(error);
        if (!write_cache(encoded, input)) {
            return false;
        }
        output = encoded.finish();
        return true;
    } catch (const std::bad_alloc&) {
        error.field = "cache";
        error.detail = "allocation failed within the bounded encoder";
        return false;
    } catch (const std::exception& exception) {
        error.field = "cache";
        error.detail = exception.what();
        return false;
    } catch (...) {
        error.field = "cache";
        error.detail = "unexpected encoder failure";
        return false;
    }
}

} // namespace gore::as::standalone::precompiled
