#pragma once

#include "gore_as_capture/format.hpp"
#include "gore_as_capture/instrumentation.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <initializer_list>

namespace gore_as_capture::v1::instrumentation::registration {

inline constexpr std::uint32_t kContractVersion = 1;
inline constexpr std::size_t kMaximumArguments = 9;
inline constexpr std::size_t kMaximumUnwindOperations = 8;

struct RegistrationTargetAddresses final {
  CaptureTargetGeneration generation{};
  std::uint32_t engine_vtable_rva{};
  std::array<std::uint32_t, 14> function_rvas{};
  std::array<std::uint32_t, 14> function_end_rvas{};
  std::array<std::uint32_t, 14> source_unwind_info_rvas{};
};

inline constexpr RegistrationTargetAddresses kRegistrationTarget24539464{
    CaptureTargetGeneration::build_24539464,
    0x081f4078,
    {0x047938b0, 0x04793fd0, 0x047997b0, 0x04799290, 0x04798f50,
     0x04798bd0, 0x047964d0, 0x04796c50, 0x0479d530, 0x04791d20,
     0x047927f0, 0x04792b10, 0x047933c0, 0x0479dc20},
    {0x04793fca, 0x047940c7, 0x04799f88, 0x0479938e, 0x04799283,
     0x04798cae, 0x0479658c, 0x047971c7, 0x0479d774, 0x04791dfb,
     0x04792b0c, 0x0479314d, 0x04793485, 0x0479dd2e},
    {0x094cbf2c, 0x094cbf70, 0x0939de28, 0x094cc078, 0x0932b838,
     0x094cc0bc, 0x094cc100, 0x0932c9a0, 0x094cc160, 0x094cb7e0,
     0x094cc180, 0x0932c9a0, 0x094cc1a0, 0x094cc200},
};

inline constexpr RegistrationTargetAddresses kRegistrationTarget24878692{
    CaptureTargetGeneration::build_24878692,
    0x081f5078,
    {0x04793870, 0x04793f90, 0x04799770, 0x04799250, 0x04798f10,
     0x04798b90, 0x04796490, 0x04796c10, 0x0479d4f0, 0x04791ce0,
     0x047927b0, 0x04792ad0, 0x04793380, 0x0479dbe0},
    {0x04793f8a, 0x04794087, 0x04799f48, 0x0479934e, 0x04799243,
     0x04798c6e, 0x0479654c, 0x04797187, 0x0479d734, 0x04791dbb,
     0x04792acc, 0x0479310d, 0x04793445, 0x0479dcee},
    {0x094cd1f0, 0x094cd234, 0x0939f0ec, 0x094cd33c, 0x0932cafc,
     0x094cd380, 0x094cd3c4, 0x0932dc64, 0x094cd424, 0x094ccaa4,
     0x094cd444, 0x0932dc64, 0x094cd464, 0x094cd4c4},
};

inline constexpr const RegistrationTargetAddresses& kRegistrationTarget =
    kRegistrationTarget24878692;
inline constexpr std::uint32_t kEngineVtableRva = kRegistrationTarget.engine_vtable_rva;
static_assert(kRegistrationTarget.generation == kCaptureTarget.generation);

enum class UnwindOperationKind : std::uint8_t {
  push_nonvolatile = 1,
  save_nonvolatile = 2,
  allocate_stack = 3,
};

// Register numbers are the AMD64 UNWIND_CODE register encodings.
enum class UnwindRegister : std::uint8_t {
  rbx = 3,
  rbp = 5,
  rsi = 6,
  rdi = 7,
  r12 = 12,
  r13 = 13,
  r14 = 14,
  r15 = 15,
};

struct RegistrationArgument final {
  std::uint8_t source{};
  std::uint8_t semantic{};
};

struct GeneratedUnwindOperation final {
  std::uint8_t code_offset{};
  UnwindOperationKind kind{};
  UnwindRegister reg{};
  std::uint8_t reserved{};
  std::uint32_t stack_offset{};
};

struct RegistrationHookPoint final {
  std::uint32_t kind{};
  std::uint32_t vtable_slot{};
  std::uint32_t function_rva{};
  std::uint8_t overwrite_bytes{};
  std::array<std::byte, 24> expected{};
  std::uint8_t argument_count{};
  std::array<RegistrationArgument, kMaximumArguments> arguments{};
  std::uint32_t contract_flags{};
  std::uint32_t source_unwind_info_rva{};
  std::uint8_t source_prolog_bytes{};
  std::uint8_t unwind_operation_count{};
  std::array<GeneratedUnwindOperation, kMaximumUnwindOperations> unwind{};
};

consteval std::array<std::byte, 24> prolog(
    const std::initializer_list<std::uint8_t> values) {
  std::array<std::byte, 24> result{};
  std::size_t index = 0;
  for (const auto value : values) result[index++] = static_cast<std::byte>(value);
  return result;
}

consteval std::array<RegistrationArgument, kMaximumArguments> arguments(
    const std::initializer_list<RegistrationArgument> values) {
  std::array<RegistrationArgument, kMaximumArguments> result{};
  std::size_t index = 0;
  for (const auto value : values) result[index++] = value;
  return result;
}

consteval std::array<GeneratedUnwindOperation, kMaximumUnwindOperations> unwind(
    const std::initializer_list<GeneratedUnwindOperation> values) {
  std::array<GeneratedUnwindOperation, kMaximumUnwindOperations> result{};
  std::size_t index = 0;
  for (const auto value : values) result[index++] = value;
  return result;
}

constexpr RegistrationArgument arg(
    const std::uint8_t source,
    const std::uint8_t semantic) noexcept {
  return {source, semantic};
}

constexpr GeneratedUnwindOperation push(
    const std::uint8_t code_offset,
    const UnwindRegister reg) noexcept {
  return {code_offset, UnwindOperationKind::push_nonvolatile, reg, 0, 0};
}

constexpr GeneratedUnwindOperation save(
    const std::uint8_t code_offset,
    const UnwindRegister reg,
    const std::uint32_t stack_offset) noexcept {
  return {code_offset, UnwindOperationKind::save_nonvolatile, reg, 0, stack_offset};
}

constexpr GeneratedUnwindOperation allocate(
    const std::uint8_t code_offset,
    const std::uint32_t stack_bytes) noexcept {
  return {code_offset, UnwindOperationKind::allocate_stack, UnwindRegister::rbx, 0,
          stack_bytes};
}

inline constexpr auto kEntryOrderAndResult =
    GORE_AS_CAPTURE_REGISTRATION_CONTRACT_ENTRY_ORDER_V1 |
    GORE_AS_CAPTURE_REGISTRATION_CONTRACT_RESULT_I32_V1;
inline constexpr auto kCallableEntry =
    kEntryOrderAndResult | GORE_AS_CAPTURE_REGISTRATION_CONTRACT_AUXILIARY_TOKEN_V1 |
    GORE_AS_CAPTURE_REGISTRATION_CONTRACT_CALLER_DESCRIPTOR_V1;

inline constexpr std::array<RegistrationHookPoint, 14> kRegistrationHooks24539464{{
    {GORE_AS_CAPTURE_REGISTRATION_GLOBAL_FUNCTION_V1,
     10,
     0x047938b0,
     14,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x55,
             0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24,
             0x30, 0xfe}),
     5,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1)}),
     kCallableEntry,
     0x094cbf2c,
     0x32,
     5,
     unwind({push(14, UnwindRegister::r12), push(12, UnwindRegister::rdi),
             push(11, UnwindRegister::rbp), save(10, UnwindRegister::rsi, 0x38),
             save(5, UnwindRegister::rbx, 0x30)})},
    {GORE_AS_CAPTURE_REGISTRATION_GLOBAL_PROPERTY_V1,
     14,
     0x04793fd0,
     16,
     prolog({0x40, 0x55, 0x56, 0x57, 0x41, 0x55, 0x41, 0x56, 0x48, 0x8d, 0xac,
             0x24, 0x80, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0x80, 0x02, 0x00,
             0x00, 0x48}),
     2,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1)}),
     kEntryOrderAndResult,
     0x094cbf70,
     0x28,
     5,
     unwind({push(8, UnwindRegister::r14), push(6, UnwindRegister::r13),
             push(4, UnwindRegister::rdi), push(3, UnwindRegister::rsi),
             push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_OBJECT_TYPE_V1,
     17,
     0x047997b0,
     17,
     prolog({0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d,
             0xac, 0x24, 0x18, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0xe8, 0x02,
             0x00, 0x00}),
     3,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1)}),
     kEntryOrderAndResult,
     0x0939de28,
     0x29,
     6,
     unwind({push(9, UnwindRegister::r15), push(7, UnwindRegister::r14),
             push(5, UnwindRegister::rdi), push(4, UnwindRegister::rsi),
             push(3, UnwindRegister::rbx), push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_OBJECT_PROPERTY_V1,
     18,
     0x04799290,
     19,
     prolog({0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57,
             0x48, 0x8d, 0xac, 0x24, 0x70, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec,
             0x90, 0x02}),
     7,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1)}),
     kEntryOrderAndResult,
     0x094cc078,
     0x2b,
     7,
     unwind({push(11, UnwindRegister::r15), push(9, UnwindRegister::r14),
             push(7, UnwindRegister::r12), push(5, UnwindRegister::rdi),
             push(4, UnwindRegister::rsi), push(3, UnwindRegister::rbx),
             push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_OBJECT_METHOD_V1,
     19,
     0x04798f50,
     19,
     prolog({0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57,
             0x48, 0x8d, 0xac, 0x24, 0xa0, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec,
             0x60, 0x02}),
     9,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1)}),
     kCallableEntry,
     0x0932b838,
     0x2b,
     7,
     unwind({push(11, UnwindRegister::r15), push(9, UnwindRegister::r14),
             push(7, UnwindRegister::r12), push(5, UnwindRegister::rdi),
             push(4, UnwindRegister::rsi), push(3, UnwindRegister::rbx),
             push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_OBJECT_BEHAVIOUR_V1,
     20,
     0x04798bd0,
     20,
     prolog({0x40, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41,
             0x57, 0x48, 0x8d, 0xac, 0x24, 0x70, 0xfe, 0xff, 0xff, 0x48, 0x81,
             0xec, 0x90}),
     9,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1)}),
     kCallableEntry,
     0x094cc0bc,
     0x2c,
     7,
     unwind({push(12, UnwindRegister::r15), push(10, UnwindRegister::r14),
             push(8, UnwindRegister::r13), push(6, UnwindRegister::r12),
             push(4, UnwindRegister::rdi), push(3, UnwindRegister::rsi),
             push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_INTERFACE_V1,
     21,
     0x047964d0,
     14,
     prolog({0x40, 0x55, 0x56, 0x57, 0x41, 0x56, 0x48, 0x8d, 0xac, 0x24, 0x68,
             0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0x98, 0x02, 0x00, 0x00, 0x48,
             0x8b, 0x05}),
     1,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x094cc100,
     0x26,
     4,
     unwind({push(6, UnwindRegister::r14), push(4, UnwindRegister::rdi),
             push(3, UnwindRegister::rsi), push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_INTERFACE_METHOD_V1,
     22,
     0x04796c50,
     16,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x20, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41,
             0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x90, 0xfe,
             0xff, 0xff}),
     2,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x0932c9a0,
     0x30,
     8,
     unwind({push(16, UnwindRegister::r15), push(14, UnwindRegister::r14),
             push(12, UnwindRegister::r13), push(10, UnwindRegister::r12),
             push(8, UnwindRegister::rdi), push(7, UnwindRegister::rsi),
             push(6, UnwindRegister::rbp), save(5, UnwindRegister::rbx, 0x58)})},
    {GORE_AS_CAPTURE_REGISTRATION_STRING_FACTORY_V1,
     25,
     0x0479d530,
     14,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x55,
             0x57, 0x41, 0x56, 0x48, 0x8d, 0xac, 0x24, 0xc0, 0xfe, 0xff, 0xff,
             0x48, 0x81}),
     2,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1)}),
     kEntryOrderAndResult,
     0x094cc160,
     0x2e,
     5,
     unwind({push(14, UnwindRegister::r14), push(12, UnwindRegister::rdi),
             push(11, UnwindRegister::rbp), save(10, UnwindRegister::rsi, 0x38),
             save(5, UnwindRegister::rbx, 0x30)})},
    {GORE_AS_CAPTURE_REGISTRATION_DEFAULT_ARRAY_TYPE_V1,
     27,
     0x04791d20,
     18,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x57,
             0x48, 0x81, 0xec, 0x40, 0x02, 0x00, 0x00, 0x48, 0x8b, 0x05, 0x8f,
             0x56, 0x2b}),
     1,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x094cb7e0,
     0x24,
     4,
     unwind({allocate(18, 0x240), push(11, UnwindRegister::rdi),
             save(10, UnwindRegister::rsi, 0x268),
             save(5, UnwindRegister::rbx, 0x260)})},
    {GORE_AS_CAPTURE_REGISTRATION_ENUM_V1,
     29,
     0x047927f0,
     14,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x55,
             0x57, 0x41, 0x56, 0x48, 0x8d, 0xac, 0x24, 0xb0, 0xfe, 0xff, 0xff,
             0x48, 0x81}),
     1,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x094cc180,
     0x2e,
     5,
     unwind({push(14, UnwindRegister::r14), push(12, UnwindRegister::rdi),
             push(11, UnwindRegister::rbp), save(10, UnwindRegister::rsi, 0x38),
             save(5, UnwindRegister::rbx, 0x30)})},
    {GORE_AS_CAPTURE_REGISTRATION_ENUM_VALUE_V1,
     30,
     0x04792b10,
     16,
     prolog({0x48, 0x89, 0x5c, 0x24, 0x20, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41,
             0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x90, 0xfe,
             0xff, 0xff}),
     3,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1)}),
     kEntryOrderAndResult,
     0x0932c9a0,
     0x30,
     8,
     unwind({push(16, UnwindRegister::r15), push(14, UnwindRegister::r14),
             push(12, UnwindRegister::r13), push(10, UnwindRegister::r12),
             push(8, UnwindRegister::rdi), push(7, UnwindRegister::rsi),
             push(6, UnwindRegister::rbp), save(5, UnwindRegister::rbx, 0x58)})},
    {GORE_AS_CAPTURE_REGISTRATION_FUNCDEF_V1,
     33,
     0x047933c0,
     15,
     prolog({0x40, 0x55, 0x53, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24,
             0x88, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0x78, 0x02, 0x00, 0x00,
             0x48, 0x8b}),
     1,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x094cc1a0,
     0x27,
     4,
     unwind({push(7, UnwindRegister::r15), push(5, UnwindRegister::r14),
             push(3, UnwindRegister::rbx), push(2, UnwindRegister::rbp)})},
    {GORE_AS_CAPTURE_REGISTRATION_TYPEDEF_V1,
     36,
     0x0479dc20,
     17,
     prolog({0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d,
             0xac, 0x24, 0x88, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0x78, 0x02,
             0x00, 0x00}),
     2,
     arguments({arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1),
                arg(GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1,
                    GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1)}),
     kEntryOrderAndResult,
     0x094cc200,
     0x29,
     6,
     unwind({push(9, UnwindRegister::r15), push(7, UnwindRegister::r14),
             push(5, UnwindRegister::rdi), push(4, UnwindRegister::rsi),
             push(3, UnwindRegister::rbx), push(2, UnwindRegister::rbp)})},
}};

consteval std::array<RegistrationHookPoint, 14> retarget_registration_hooks(
    const std::array<RegistrationHookPoint, 14>& source,
    const RegistrationTargetAddresses& target) {
  auto result = source;
  for (std::size_t index = 0; index < result.size(); ++index) {
    result[index].function_rva = target.function_rvas[index];
    result[index].source_unwind_info_rva = target.source_unwind_info_rvas[index];
  }
  return result;
}

inline constexpr auto kPinnedRegistrationHooks =
    retarget_registration_hooks(kRegistrationHooks24539464, kRegistrationTarget);

static_assert([] {
  for (std::size_t index = 0; index < kPinnedRegistrationHooks.size(); ++index) {
    if (kPinnedRegistrationHooks[index].function_rva !=
            kRegistrationTarget.function_rvas[index] ||
        kPinnedRegistrationHooks[index].source_unwind_info_rva !=
            kRegistrationTarget.source_unwind_info_rvas[index] ||
        kRegistrationTarget.function_rvas[index] >=
            kRegistrationTarget.function_end_rvas[index] ||
        kRegistrationTarget.function_end_rvas[index] > kPeSizeOfImage ||
        kRegistrationTarget.source_unwind_info_rvas[index] >= kPeSizeOfImage) {
      return false;
    }
  }
  return true;
}());

static_assert([] {
  for (std::size_t index = 0; index < kRegistrationHooks24539464.size(); ++index) {
    if (kRegistrationHooks24539464[index].function_rva !=
            kRegistrationTarget24539464.function_rvas[index] ||
        kRegistrationHooks24539464[index].source_unwind_info_rva !=
            kRegistrationTarget24539464.source_unwind_info_rvas[index]) {
      return false;
    }
  }
  return true;
}());

consteval std::uint64_t fingerprint(const bool include_prologs) {
  std::uint64_t hash = 14695981039346656037ull;
  const auto append = [&hash](const std::uint8_t value) {
    hash ^= value;
    hash *= 1099511628211ull;
  };
  const auto append_u32 = [&append](const std::uint32_t value) {
    for (unsigned shift = 0; shift < 32; shift += 8) {
      append(static_cast<std::uint8_t>((value >> shift) & 0xffu));
    }
  };
  append_u32(kContractVersion);
  append_u32(kEngineVtableRva);
  for (const auto& point : kPinnedRegistrationHooks) {
    append_u32(point.kind);
    append_u32(point.vtable_slot);
    append_u32(point.function_rva);
    append(point.overwrite_bytes);
    append(point.argument_count);
    append_u32(point.contract_flags);
    append_u32(point.source_unwind_info_rva);
    append(point.source_prolog_bytes);
    append(point.unwind_operation_count);
    for (std::size_t index = 0; index < point.argument_count; ++index) {
      append(point.arguments[index].source);
      append(point.arguments[index].semantic);
    }
    if (include_prologs) {
      for (std::size_t index = 0; index < point.overwrite_bytes; ++index) {
        append(std::to_integer<std::uint8_t>(point.expected[index]));
      }
    }
  }
  return hash;
}

inline constexpr std::uint64_t kRegistrationTableFingerprint = fingerprint(false);
inline constexpr std::uint64_t kRegistrationPrologFingerprint = fingerprint(true);
inline constexpr std::uint32_t kAllRegistrationHookMask =
    (1u << kPinnedRegistrationHooks.size()) - 1u;

static_assert(kPinnedRegistrationHooks.size() == 14);
static_assert([] {
  std::uint32_t previous_slot = 0;
  for (const auto& point : kPinnedRegistrationHooks) {
    if (point.kind == 0 || point.vtable_slot <= previous_slot || point.function_rva == 0 ||
        point.overwrite_bytes < 14 || point.overwrite_bytes > point.expected.size() ||
        point.argument_count == 0 || point.argument_count > point.arguments.size() ||
        point.source_unwind_info_rva == 0 || point.source_prolog_bytes < point.overwrite_bytes ||
        point.unwind_operation_count == 0 ||
        point.unwind_operation_count > point.unwind.size()) {
      return false;
    }
    previous_slot = point.vtable_slot;
  }
  return true;
}());

}  // namespace gore_as_capture::v1::instrumentation::registration
