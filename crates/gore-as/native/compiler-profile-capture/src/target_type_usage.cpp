#include "target_type_usage.hpp"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <span>
#include <string_view>
#include <type_traits>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::uint32_t kFromTypeIdRva = 0x0474d8f0;
constexpr std::uint32_t kDestroyTypeUsageRva = 0x0465c0d0;
constexpr std::array<std::byte, 19> kFromTypeIdProlog{
    std::byte{0x48}, std::byte{0x89}, std::byte{0x5c}, std::byte{0x24}, std::byte{0x18},
    std::byte{0x55}, std::byte{0x56}, std::byte{0x57}, std::byte{0x41}, std::byte{0x54},
    std::byte{0x41}, std::byte{0x55}, std::byte{0x41}, std::byte{0x56}, std::byte{0x41},
    std::byte{0x57}, std::byte{0x48}, std::byte{0x8d}, std::byte{0xac}};
constexpr std::array<std::byte, 15> kDestroyTypeUsageProlog{
    std::byte{0x48}, std::byte{0x89}, std::byte{0x5c}, std::byte{0x24}, std::byte{0x08},
    std::byte{0x48}, std::byte{0x89}, std::byte{0x74}, std::byte{0x24}, std::byte{0x10},
    std::byte{0x57}, std::byte{0x48}, std::byte{0x83}, std::byte{0xec}, std::byte{0x20}};
constexpr std::uint32_t kMaximumTypeUsageDepth = 64;
constexpr std::uint32_t kMaximumTypeUsageNodes = 4096;
constexpr std::int32_t kMaximumValueBytes = 64 * 1024 * 1024;

// FAngelscriptType vtable byte offsets. The class-generator loop at target
// 0x485e281/0x485e2a1/0x485e309 independently witnesses the property-policy
// slots; the remaining slots are witnessed by the TSet/TMap/TOptional
// operation validators at 0x4834e90..0x483543c and 0x484ddc9.
namespace type_slot {
constexpr std::size_t can_create_property = 0x048 / sizeof(std::uintptr_t);
constexpr std::size_t never_requires_gc = 0x070 / sizeof(std::uintptr_t);
constexpr std::size_t requires_property = 0x078 / sizeof(std::uintptr_t);
constexpr std::size_t is_object_pointer = 0x080 / sizeof(std::uintptr_t);
constexpr std::size_t can_be_template_subtype = 0x090 / sizeof(std::uintptr_t);
constexpr std::size_t can_copy = 0x0a8 / sizeof(std::uintptr_t);
constexpr std::size_t need_copy = 0x0b0 / sizeof(std::uintptr_t);
constexpr std::size_t can_compare = 0x0c0 / sizeof(std::uintptr_t);
constexpr std::size_t can_construct = 0x0d0 / sizeof(std::uintptr_t);
constexpr std::size_t need_construct = 0x0d8 / sizeof(std::uintptr_t);
constexpr std::size_t value_size = 0x0e8 / sizeof(std::uintptr_t);
constexpr std::size_t can_destruct = 0x0f0 / sizeof(std::uintptr_t);
constexpr std::size_t need_destruct = 0x0f8 / sizeof(std::uintptr_t);
constexpr std::size_t can_hash_value = 0x148 / sizeof(std::uintptr_t);
constexpr std::size_t value_alignment = 0x158 / sizeof(std::uintptr_t);
}  // namespace type_slot

bool readable_range(const std::uintptr_t first, const std::size_t bytes) noexcept {
  if (first == 0 || bytes == 0 || first > std::numeric_limits<std::uintptr_t>::max() - bytes) {
    return false;
  }
  auto cursor = first;
  const auto end = first + bytes;
  while (cursor < end) {
    MEMORY_BASIC_INFORMATION region{};
    if (VirtualQuery(reinterpret_cast<const void*>(cursor), &region, sizeof(region)) !=
            sizeof(region) ||
        region.State != MEM_COMMIT || (region.Protect & PAGE_GUARD) != 0) {
      return false;
    }
    const DWORD protection = region.Protect & 0xffu;
    if (protection != PAGE_READONLY && protection != PAGE_READWRITE &&
        protection != PAGE_WRITECOPY && protection != PAGE_EXECUTE_READ &&
        protection != PAGE_EXECUTE_READWRITE && protection != PAGE_EXECUTE_WRITECOPY) {
      return false;
    }
    const auto base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
    if (base > std::numeric_limits<std::uintptr_t>::max() - region.RegionSize) return false;
    const auto next = base + region.RegionSize;
    if (next <= cursor) return false;
    cursor = std::min(next, end);
  }
  return true;
}

bool image_address(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t address) noexcept {
  return image != 0 && address >= image && address - image < image_bytes;
}

template <typename Return, typename... Arguments>
TargetTypeUsageError invoke_type_slot(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    const std::size_t slot,
    Return& output,
    Arguments... arguments) noexcept {
  static_assert(!std::is_void_v<Return>);
  if (usage.type == 0 || !readable_range(usage.type, sizeof(std::uintptr_t))) {
    return TargetTypeUsageError::unresolved_type;
  }
  std::uintptr_t vtable = 0;
  std::memcpy(&vtable, reinterpret_cast<const void*>(usage.type), sizeof(vtable));
  if (!readable_range(vtable + slot * sizeof(std::uintptr_t), sizeof(std::uintptr_t))) {
    return TargetTypeUsageError::unreadable_value;
  }
  std::uintptr_t target = 0;
  std::memcpy(
      &target,
      reinterpret_cast<const void*>(vtable + slot * sizeof(std::uintptr_t)),
      sizeof(target));
  if (!image_address(image, image_bytes, target)) {
    return TargetTypeUsageError::abi_target_outside_image;
  }
  using Function = Return(__fastcall*)(std::uintptr_t, Arguments...);
  Function function = nullptr;
  static_assert(sizeof(function) == sizeof(target));
  std::memcpy(&function, &target, sizeof(function));
  try {
    output = function(usage.type, arguments...);
  } catch (...) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  return TargetTypeUsageError::ok;
}

TargetTypeUsageError boolean_operation(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    const std::size_t slot,
    const bool takes_usage,
    bool& output) noexcept {
  std::uint8_t value = 0;
  const auto status = takes_usage
                          ? invoke_type_slot(
                                image,
                                image_bytes,
                                usage,
                                slot,
                                value,
                                &usage)
                          : invoke_type_slot(
                                image, image_bytes, usage, slot, value);
  if (status != TargetTypeUsageError::ok) return status;
  if (value > 1) return TargetTypeUsageError::invalid_operation_value;
  output = value != 0;
  return TargetTypeUsageError::ok;
}

bool pinned_prolog(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uint32_t rva,
    const std::span<const std::byte> expected) noexcept {
  if (rva >= image_bytes || expected.size() > image_bytes - rva ||
      !readable_range(image + rva, expected.size())) {
    return false;
  }
  return std::memcmp(
             reinterpret_cast<const void*>(image + rva),
             expected.data(),
             expected.size()) == 0;
}

TargetTypeUsageError validate_type_usage_tree(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    const std::uint32_t depth,
    std::uint32_t& nodes,
    FixedTypeOperationsProjection& root) noexcept {
  if (depth > kMaximumTypeUsageDepth || nodes == kMaximumTypeUsageNodes) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  ++nodes;
  FixedTypeOperationsProjection operations{};
  const auto status = project_target_type_operations_v1(
      image, image_bytes, usage, operations);
  if (status != TargetTypeUsageError::ok) return status;
  if (depth == 0) root = operations;
  for (std::int32_t index = 0; index < usage.subtype_count; ++index) {
    TargetTypeUsage subtype{};
    std::memcpy(
        &subtype,
        reinterpret_cast<const void*>(
            usage.subtypes + static_cast<std::size_t>(index) * sizeof(subtype)),
        sizeof(subtype));
    const auto subtype_status = validate_type_usage_tree(
        image, image_bytes, subtype, depth + 1, nodes, root);
    if (subtype_status != TargetTypeUsageError::ok) return subtype_status;
  }
  return TargetTypeUsageError::ok;
}

std::uint32_t expected_container_subtypes(const TypeOperationsJsonKind kind) noexcept {
  switch (kind) {
    case TypeOperationsJsonKind::t_array:
    case TypeOperationsJsonKind::t_set:
    case TypeOperationsJsonKind::t_optional:
      return 1;
    case TypeOperationsJsonKind::t_map:
      return 2;
    case TypeOperationsJsonKind::fixed:
    case TypeOperationsJsonKind::unavailable:
      return 0;
  }
  return std::numeric_limits<std::uint32_t>::max();
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
std::uint8_t __fastcall fixture_true(std::uintptr_t, const TargetTypeUsage*) noexcept {
  return 1;
}
std::uint8_t __fastcall fixture_false(std::uintptr_t, const TargetTypeUsage*) noexcept {
  return 0;
}
std::int32_t __fastcall fixture_size(std::uintptr_t, const TargetTypeUsage*) noexcept {
  return 24;
}
std::int32_t __fastcall fixture_alignment(std::uintptr_t, const TargetTypeUsage*) noexcept {
  return 8;
}
#endif

}  // namespace

TargetTypeUsageError project_target_type_operations_v1(
    const std::uintptr_t primary_image,
    const std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    FixedTypeOperationsProjection& projection) noexcept {
  if (primary_image == 0 || image_bytes == 0 || usage.subtype_count < 0 ||
      usage.subtype_capacity < usage.subtype_count || usage.is_reference > 1 ||
      usage.is_const > 1 ||
      (usage.subtype_count != 0 &&
       (usage.subtypes == 0 ||
        static_cast<std::uint32_t>(usage.subtype_count) >
            std::numeric_limits<std::size_t>::max() / sizeof(TargetTypeUsage) ||
        !readable_range(
            usage.subtypes,
            static_cast<std::size_t>(usage.subtype_count) * sizeof(TargetTypeUsage))))) {
    return TargetTypeUsageError::invalid_argument;
  }

  FixedTypeOperationsProjection value{};
#define GORE_AS_TYPE_BOOL(field, slot, takes_usage)                                     \
  do {                                                                                  \
    const auto status = boolean_operation(                                              \
        primary_image, image_bytes, usage, slot, takes_usage, value.field);             \
    if (status != TargetTypeUsageError::ok) return status;                               \
  } while (false)
  GORE_AS_TYPE_BOOL(can_create_property, type_slot::can_create_property, true);
  GORE_AS_TYPE_BOOL(never_requires_gc, type_slot::never_requires_gc, true);
  GORE_AS_TYPE_BOOL(requires_property, type_slot::requires_property, true);
  GORE_AS_TYPE_BOOL(can_be_template_subtype, type_slot::can_be_template_subtype, false);
  GORE_AS_TYPE_BOOL(can_construct, type_slot::can_construct, true);
  GORE_AS_TYPE_BOOL(need_construct, type_slot::need_construct, true);
  GORE_AS_TYPE_BOOL(can_destruct, type_slot::can_destruct, true);
  GORE_AS_TYPE_BOOL(need_destruct, type_slot::need_destruct, true);
  GORE_AS_TYPE_BOOL(can_copy, type_slot::can_copy, true);
  GORE_AS_TYPE_BOOL(need_copy, type_slot::need_copy, true);
  GORE_AS_TYPE_BOOL(can_compare, type_slot::can_compare, true);
  GORE_AS_TYPE_BOOL(can_hash_value, type_slot::can_hash_value, true);
  GORE_AS_TYPE_BOOL(is_object_pointer, type_slot::is_object_pointer, false);
#undef GORE_AS_TYPE_BOOL

  std::int32_t size = 0;
  auto status = invoke_type_slot(
      primary_image, image_bytes, usage, type_slot::value_size, size, &usage);
  if (status != TargetTypeUsageError::ok) return status;
  std::int32_t alignment = 0;
  status = invoke_type_slot(
      primary_image, image_bytes, usage, type_slot::value_alignment, alignment, &usage);
  if (status != TargetTypeUsageError::ok) return status;
  if (size <= 0 || size > kMaximumValueBytes || alignment <= 0 || alignment > 4096 ||
      (alignment & (alignment - 1)) != 0) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  value.value_size = static_cast<std::uint32_t>(size);
  value.value_alignment = static_cast<std::uint32_t>(alignment);
  projection = value;
  return TargetTypeUsageError::ok;
}

TargetTypeUsageError resolve_target_type_operations_v1(
    const std::uintptr_t primary_image,
    const std::uint32_t image_bytes,
    const std::int32_t engine_type_id,
    FixedTypeOperationsProjection& projection) noexcept {
  if (primary_image == 0 || image_bytes == 0 || engine_type_id == 0) {
    return TargetTypeUsageError::invalid_argument;
  }
  if (!pinned_prolog(
          primary_image, image_bytes, kFromTypeIdRva, kFromTypeIdProlog) ||
      !pinned_prolog(
          primary_image,
          image_bytes,
          kDestroyTypeUsageRva,
          kDestroyTypeUsageProlog)) {
    return TargetTypeUsageError::prolog_drift;
  }
  using FromTypeId = void(__fastcall*)(TargetTypeUsage*, std::int32_t);
  using DestroyTypeUsage = void(__fastcall*)(TargetTypeUsage*);
  const auto from_address = primary_image + kFromTypeIdRva;
  const auto destroy_address = primary_image + kDestroyTypeUsageRva;
  FromTypeId from_type_id = nullptr;
  DestroyTypeUsage destroy = nullptr;
  std::memcpy(&from_type_id, &from_address, sizeof(from_type_id));
  std::memcpy(&destroy, &destroy_address, sizeof(destroy));

  TargetTypeUsage usage{};
  try {
    from_type_id(&usage, engine_type_id);
  } catch (...) {
    return TargetTypeUsageError::unresolved_type;
  }
  const auto status = usage.type == 0
                          ? TargetTypeUsageError::unresolved_type
                          : project_target_type_operations_v1(
                                primary_image, image_bytes, usage, projection);
  try {
    destroy(&usage);
  } catch (...) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  return status;
}

TargetTypeUsageError project_target_type_operations_tree_v1(
    const std::uintptr_t primary_image,
    const std::uint32_t image_bytes,
    const TargetTypeUsage& usage,
    const char* const declaration,
    const std::size_t declaration_bytes,
    TargetTypeOperationsProjection& projection) noexcept {
  if (primary_image == 0 || image_bytes == 0 || declaration == nullptr ||
      declaration_bytes == 0 || usage.subtype_count < 0) {
    return TargetTypeUsageError::invalid_argument;
  }
  TargetTypeOperationsProjection value{};
  std::uint32_t nodes = 0;
  auto status = validate_type_usage_tree(
      primary_image, image_bytes, usage, 0, nodes, value.fixed);
  if (status != TargetTypeUsageError::ok) return status;
  value.kind = classify_target_type_operations_v1(declaration, declaration_bytes);
  value.subtype_count = static_cast<std::uint32_t>(usage.subtype_count);
  const auto expected = expected_container_subtypes(value.kind);
  if ((expected != 0 && value.subtype_count != expected) ||
      expected == std::numeric_limits<std::uint32_t>::max()) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  projection = value;
  return TargetTypeUsageError::ok;
}

TargetTypeUsageError resolve_target_type_operations_projection_v1(
    const std::uintptr_t primary_image,
    const std::uint32_t image_bytes,
    const std::int32_t engine_type_id,
    const char* const declaration,
    const std::size_t declaration_bytes,
    TargetTypeOperationsProjection& projection) noexcept {
  if (primary_image == 0 || image_bytes == 0 || engine_type_id == 0 ||
      declaration == nullptr || declaration_bytes == 0) {
    return TargetTypeUsageError::invalid_argument;
  }
  if (!pinned_prolog(
          primary_image, image_bytes, kFromTypeIdRva, kFromTypeIdProlog) ||
      !pinned_prolog(
          primary_image,
          image_bytes,
          kDestroyTypeUsageRva,
          kDestroyTypeUsageProlog)) {
    return TargetTypeUsageError::prolog_drift;
  }
  using FromTypeId = void(__fastcall*)(TargetTypeUsage*, std::int32_t);
  using DestroyTypeUsage = void(__fastcall*)(TargetTypeUsage*);
  const auto from_address = primary_image + kFromTypeIdRva;
  const auto destroy_address = primary_image + kDestroyTypeUsageRva;
  FromTypeId from_type_id = nullptr;
  DestroyTypeUsage destroy = nullptr;
  std::memcpy(&from_type_id, &from_address, sizeof(from_type_id));
  std::memcpy(&destroy, &destroy_address, sizeof(destroy));

  TargetTypeUsage usage{};
  try {
    from_type_id(&usage, engine_type_id);
  } catch (...) {
    return TargetTypeUsageError::unresolved_type;
  }
  TargetTypeOperationsProjection value{};
  TargetTypeUsageError status = TargetTypeUsageError::unresolved_type;
  if (usage.type != 0) {
    status = project_target_type_operations_tree_v1(
        primary_image, image_bytes, usage, declaration, declaration_bytes, value);
  }
  try {
    destroy(&usage);
  } catch (...) {
    return TargetTypeUsageError::invalid_operation_value;
  }
  if (status == TargetTypeUsageError::ok) projection = value;
  return status;
}

TypeOperationsJsonKind classify_target_type_operations_v1(
    const char* const declaration,
    const std::size_t declaration_bytes) noexcept {
  if (declaration == nullptr || declaration_bytes == 0) {
    return TypeOperationsJsonKind::unavailable;
  }
  const std::string_view value(declaration, declaration_bytes);
  if (value.starts_with("TArray<")) return TypeOperationsJsonKind::t_array;
  if (value.starts_with("TMap<")) return TypeOperationsJsonKind::t_map;
  if (value.starts_with("TSet<")) return TypeOperationsJsonKind::t_set;
  if (value.starts_with("TOptional<")) return TypeOperationsJsonKind::t_optional;
  return TypeOperationsJsonKind::fixed;
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_type_usage_selftest_v1() noexcept {
  HMODULE module = nullptr;
  if (GetModuleHandleExW(
          GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
              GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
          reinterpret_cast<LPCWSTR>(&target_type_usage_selftest_v1),
          &module) == FALSE ||
      module == nullptr) {
    return false;
  }
  const auto image = reinterpret_cast<std::uintptr_t>(module);
  const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(image);
  if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0) return false;
  const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS64*>(image + dos->e_lfanew);
  if (nt->Signature != IMAGE_NT_SIGNATURE) return false;

  std::array<std::uintptr_t, type_slot::value_alignment + 1> vtable{};
  const auto true_address = reinterpret_cast<std::uintptr_t>(&fixture_true);
  const auto false_address = reinterpret_cast<std::uintptr_t>(&fixture_false);
  vtable.fill(false_address);
  vtable[type_slot::can_create_property] = true_address;
  vtable[type_slot::never_requires_gc] = false_address;
  vtable[type_slot::requires_property] = true_address;
  vtable[type_slot::can_be_template_subtype] = true_address;
  vtable[type_slot::can_construct] = true_address;
  vtable[type_slot::need_construct] = false_address;
  vtable[type_slot::can_destruct] = true_address;
  vtable[type_slot::need_destruct] = false_address;
  vtable[type_slot::can_copy] = true_address;
  vtable[type_slot::need_copy] = false_address;
  vtable[type_slot::can_compare] = true_address;
  vtable[type_slot::can_hash_value] = true_address;
  vtable[type_slot::is_object_pointer] = false_address;
  vtable[type_slot::value_size] = reinterpret_cast<std::uintptr_t>(&fixture_size);
  vtable[type_slot::value_alignment] = reinterpret_cast<std::uintptr_t>(&fixture_alignment);
  const std::uintptr_t type_vtable = reinterpret_cast<std::uintptr_t>(vtable.data());
  TargetTypeUsage usage{};
  usage.type = reinterpret_cast<std::uintptr_t>(&type_vtable);
  TargetTypeUsage subtype = usage;
  usage.subtypes = reinterpret_cast<std::uintptr_t>(&subtype);
  usage.subtype_count = 1;
  usage.subtype_capacity = 1;
  FixedTypeOperationsProjection operations{};
  TargetTypeOperationsProjection tree{};
  if (project_target_type_operations_v1(
          image,
          nt->OptionalHeader.SizeOfImage,
          usage,
          operations) != TargetTypeUsageError::ok ||
      !operations.can_create_property || operations.never_requires_gc ||
      !operations.requires_property || !operations.can_be_template_subtype ||
      !operations.can_construct ||
      operations.need_construct || !operations.can_destruct || operations.need_destruct ||
      !operations.can_copy || operations.need_copy || !operations.can_compare ||
      !operations.can_hash_value || operations.value_size != 24 ||
      operations.value_alignment != 8 || operations.is_object_pointer ||
      project_target_type_operations_tree_v1(
          image,
          nt->OptionalHeader.SizeOfImage,
          usage,
          "TArray<class T>",
          15,
          tree) != TargetTypeUsageError::ok ||
      tree.kind != TypeOperationsJsonKind::t_array || tree.subtype_count != 1 ||
      classify_target_type_operations_v1("TArray<class T>", 15) !=
          TypeOperationsJsonKind::t_array ||
      classify_target_type_operations_v1("TMap<class K, class V>", 22) !=
          TypeOperationsJsonKind::t_map ||
      classify_target_type_operations_v1("FVector", 7) !=
          TypeOperationsJsonKind::fixed) {
    return false;
  }
  vtable[type_slot::value_alignment] = image + nt->OptionalHeader.SizeOfImage;
  return project_target_type_operations_v1(
             image,
             nt->OptionalHeader.SizeOfImage,
             usage,
             operations) == TargetTypeUsageError::abi_target_outside_image;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
