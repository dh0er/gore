#include "target_snapshot.hpp"

#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <type_traits>
#include <vector>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::uint32_t kMaxItems = 2'000'000;
constexpr std::size_t kMaxStringBytes = 64 * 1024;

namespace engine_slot {
constexpr std::size_t global_function_count = 11;
constexpr std::size_t global_function_by_index = 12;
constexpr std::size_t global_property_count = 15;
constexpr std::size_t global_property_by_index = 16;
constexpr std::size_t object_type_count = 23;
constexpr std::size_t object_type_by_index = 24;
constexpr std::size_t string_factory_return_type = 26;
constexpr std::size_t default_array_type = 28;
constexpr std::size_t enum_count = 31;
constexpr std::size_t enum_by_index = 32;
constexpr std::size_t funcdef_count = 34;
constexpr std::size_t funcdef_by_index = 35;
constexpr std::size_t typedef_count = 37;
constexpr std::size_t typedef_by_index = 38;
}  // namespace engine_slot

namespace type_slot {
constexpr std::size_t config_group = 1;
constexpr std::size_t access_mask = 2;
constexpr std::size_t name = 6;
constexpr std::size_t name_space = 7;
constexpr std::size_t base_type = 8;
constexpr std::size_t flags = 11;
constexpr std::size_t size = 12;
constexpr std::size_t type_id = 13;
constexpr std::size_t interface_count = 17;
constexpr std::size_t interface_by_index = 18;
constexpr std::size_t factory_count = 20;
constexpr std::size_t factory_by_index = 21;
constexpr std::size_t method_count = 23;
constexpr std::size_t method_by_index = 24;
constexpr std::size_t property_count = 27;
constexpr std::size_t property = 28;
constexpr std::size_t property_declaration = 30;
constexpr std::size_t behaviour_count = 31;
constexpr std::size_t behaviour_by_index = 32;
constexpr std::size_t enum_value_count = 36;
constexpr std::size_t enum_value_by_index = 37;
constexpr std::size_t typedef_type_id = 38;
constexpr std::size_t funcdef_signature = 39;
constexpr std::size_t get_user_data = 41;
}  // namespace type_slot

namespace function_slot {
constexpr std::size_t id = 3;
constexpr std::size_t config_group = 8;
constexpr std::size_t access_mask = 9;
constexpr std::size_t name = 14;
constexpr std::size_t name_space = 15;
constexpr std::size_t declaration = 16;
}  // namespace function_slot

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
  return address >= image && address - image < image_bytes;
}

template <typename Return, typename... Arguments>
SnapshotError invoke_slot(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t object,
    const std::size_t slot,
    Return& result,
    Arguments... arguments) noexcept {
  static_assert(!std::is_void_v<Return>);
  if (!readable_range(object, sizeof(std::uintptr_t))) return SnapshotError::unreadable_object;
  std::uintptr_t vtable = 0;
  std::memcpy(&vtable, reinterpret_cast<const void*>(object), sizeof(vtable));
  if (!image_address(image, image_bytes, vtable) ||
      !readable_range(vtable + slot * sizeof(std::uintptr_t), sizeof(std::uintptr_t))) {
    return SnapshotError::abi_target_outside_image;
  }
  std::uintptr_t target = 0;
  std::memcpy(
      &target,
      reinterpret_cast<const void*>(vtable + slot * sizeof(std::uintptr_t)),
      sizeof(target));
  if (!image_address(image, image_bytes, target)) {
    return SnapshotError::abi_target_outside_image;
  }
  using Function = Return(__fastcall*)(std::uintptr_t, Arguments...);
  static_assert(sizeof(Function) == sizeof(target));
  Function function = nullptr;
  std::memcpy(&function, &target, sizeof(function));
  try {
    result = function(object, arguments...);
    return SnapshotError::ok;
  } catch (...) {
    return SnapshotError::invalid_value;
  }
}

class Sha256 final {
 public:
  Sha256() noexcept {
    if (BCryptOpenAlgorithmProvider(
            &algorithm_, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0) {
      return;
    }
    DWORD result_bytes = 0;
    if (BCryptGetProperty(
            algorithm_,
            BCRYPT_OBJECT_LENGTH,
            reinterpret_cast<PUCHAR>(&object_bytes_),
            sizeof(object_bytes_),
            &result_bytes,
            0) < 0 ||
        result_bytes != sizeof(object_bytes_) || object_bytes_ == 0 ||
        object_bytes_ > 64 * 1024) {
      return;
    }
    try {
      object_.resize(object_bytes_);
    } catch (...) {
      return;
    }
    if (BCryptCreateHash(
            algorithm_, hash_.address(), object_.data(), object_bytes_, nullptr, 0, 0) < 0) {
      return;
    }
    valid_ = true;
  }

  ~Sha256() {
    if (hash_.value != nullptr) (void)BCryptDestroyHash(hash_.value);
    if (algorithm_ != nullptr) (void)BCryptCloseAlgorithmProvider(algorithm_, 0);
  }
  Sha256(const Sha256&) = delete;
  Sha256& operator=(const Sha256&) = delete;

  bool append(const void* const bytes, const std::size_t size) noexcept {
    if (!valid_ || (size != 0 && bytes == nullptr) || size > std::numeric_limits<ULONG>::max()) {
      return false;
    }
    return BCryptHashData(
               hash_.value,
               const_cast<PUCHAR>(static_cast<const UCHAR*>(bytes)),
               static_cast<ULONG>(size),
               0) >= 0;
  }

  bool u32(const std::uint32_t value) noexcept { return append(&value, sizeof(value)); }

  bool string(const char* const value) noexcept {
    if (value == nullptr) return u32(0xffff'ffffu);
    std::size_t length = 0;
    while (length < kMaxStringBytes) {
      const auto address = reinterpret_cast<std::uintptr_t>(value) + length;
      if (!readable_range(address, 1)) return false;
      if (value[length] == '\0') break;
      ++length;
    }
    if (length == kMaxStringBytes || length > std::numeric_limits<std::uint32_t>::max()) {
      return false;
    }
    return u32(static_cast<std::uint32_t>(length)) && append(value, length);
  }

  bool finish(Digest& digest) noexcept {
    if (!valid_ ||
        BCryptFinishHash(
            hash_.value,
            reinterpret_cast<PUCHAR>(digest.data()),
            static_cast<ULONG>(digest.size()),
            0) < 0) {
      return false;
    }
    valid_ = false;
    return true;
  }

 private:
  struct HashHandle final {
    BCRYPT_HASH_HANDLE value{};
    BCRYPT_HASH_HANDLE* address() noexcept { return &value; }
  };
  BCRYPT_ALG_HANDLE algorithm_{};
  HashHandle hash_{};
  DWORD object_bytes_{};
  std::vector<UCHAR> object_;
  bool valid_{};
};

SnapshotError hash_function(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t function,
    Sha256& hash) noexcept {
  if (function == 0) return SnapshotError::invalid_value;
  std::int32_t id = 0;
  std::uint32_t access_mask = 0;
  const char* config_group = nullptr;
  const char* name = nullptr;
  const char* name_space = nullptr;
  const char* declaration = nullptr;
#define GORE_AS_SNAPSHOT_INVOKE(slot, output, ...)                                  \
  do {                                                                              \
    const auto status = invoke_slot(                                                 \
        image, image_bytes, function, slot, output __VA_OPT__(, ) __VA_ARGS__);     \
    if (status != SnapshotError::ok) return status;                                  \
  } while (false)
  GORE_AS_SNAPSHOT_INVOKE(function_slot::id, id);
  GORE_AS_SNAPSHOT_INVOKE(function_slot::config_group, config_group);
  GORE_AS_SNAPSHOT_INVOKE(function_slot::access_mask, access_mask);
  GORE_AS_SNAPSHOT_INVOKE(function_slot::name, name);
  GORE_AS_SNAPSHOT_INVOKE(function_slot::name_space, name_space);
  GORE_AS_SNAPSHOT_INVOKE(function_slot::declaration, declaration, true, true, false, true);
#undef GORE_AS_SNAPSHOT_INVOKE
  if (!hash.u32(static_cast<std::uint32_t>(id)) || !hash.u32(access_mask) ||
      !hash.string(config_group) || !hash.string(name) || !hash.string(name_space) ||
      !hash.string(declaration)) {
    return SnapshotError::hash_failure;
  }
  return SnapshotError::ok;
}

bool add_count(std::uint32_t& value, const std::uint32_t add) noexcept {
  if (add > kMaxItems || value > kMaxItems - add) return false;
  value += add;
  return true;
}

SnapshotError copy_public_string(const char* const value, std::string& output) noexcept {
  if (value == nullptr) return SnapshotError::invalid_value;
  std::size_t length = 0;
  while (length < kMaxStringBytes) {
    const auto address = reinterpret_cast<std::uintptr_t>(value) + length;
    if (!readable_range(address, 1)) return SnapshotError::unreadable_object;
    if (value[length] == '\0') break;
    ++length;
  }
  if (length == kMaxStringBytes) return SnapshotError::limit_exceeded;
  try {
    output.assign(value, length);
  } catch (...) {
    return SnapshotError::limit_exceeded;
  }
  return SnapshotError::ok;
}

SnapshotError hash_type(
    const std::uintptr_t image,
    const std::uint32_t image_bytes,
    const std::uintptr_t type,
    RegistryCounts& counts,
    Sha256& hash) noexcept {
  if (type == 0 || !readable_range(type + sizeof(std::uintptr_t), sizeof(std::int32_t))) {
    return SnapshotError::unreadable_object;
  }
  const char* config_group = nullptr;
  const char* name = nullptr;
  const char* name_space = nullptr;
  std::uint32_t access_mask = 0;
  std::uint32_t flags = 0;
  std::uint32_t size = 0;
  std::int32_t type_id = 0;
  std::uint32_t alignment = 0;
  std::memcpy(
      &alignment,
      reinterpret_cast<const void*>(type + sizeof(std::uintptr_t)),
      sizeof(alignment));
#define GORE_AS_SNAPSHOT_INVOKE(slot, output, ...)                              \
  do {                                                                          \
    const auto status = invoke_slot(                                             \
        image, image_bytes, type, slot, output __VA_OPT__(, ) __VA_ARGS__);     \
    if (status != SnapshotError::ok) return status;                              \
  } while (false)
  GORE_AS_SNAPSHOT_INVOKE(type_slot::config_group, config_group);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::access_mask, access_mask);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::name, name);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::name_space, name_space);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::flags, flags);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::size, size);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::type_id, type_id);
#undef GORE_AS_SNAPSHOT_INVOKE
  if (alignment == 0 || alignment > 4096 || (alignment & (alignment - 1)) != 0 ||
      !hash.u32(static_cast<std::uint32_t>(type_id)) || !hash.u32(flags) ||
      !hash.u32(size) || !hash.u32(alignment) || !hash.u32(access_mask) ||
      !hash.string(config_group) || !hash.string(name) || !hash.string(name_space)) {
    return SnapshotError::invalid_value;
  }

  std::uintptr_t base_type = 0;
  std::uint32_t interface_count = 0;
#define GORE_AS_SNAPSHOT_INVOKE(slot, output, ...)                              \
  do {                                                                          \
    const auto status = invoke_slot(                                             \
        image, image_bytes, type, slot, output __VA_OPT__(, ) __VA_ARGS__);     \
    if (status != SnapshotError::ok) return status;                              \
  } while (false)
  GORE_AS_SNAPSHOT_INVOKE(type_slot::base_type, base_type);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::interface_count, interface_count);
#undef GORE_AS_SNAPSHOT_INVOKE
  if (interface_count > kMaxItems || !hash.u32(interface_count)) {
    return SnapshotError::limit_exceeded;
  }
  if (base_type == 0) {
    if (!hash.u32(0xffff'ffffu)) return SnapshotError::hash_failure;
  } else {
    std::int32_t base_type_id = 0;
    const auto status = invoke_slot(
        image, image_bytes, base_type, type_slot::type_id, base_type_id);
    if (status != SnapshotError::ok ||
        !hash.u32(static_cast<std::uint32_t>(base_type_id))) {
      return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
    }
  }
  for (std::uint32_t index = 0; index < interface_count; ++index) {
    std::uintptr_t interface_type = 0;
    auto status = invoke_slot(
        image,
        image_bytes,
        type,
        type_slot::interface_by_index,
        interface_type,
        index);
    if (status != SnapshotError::ok || interface_type == 0) {
      return status == SnapshotError::ok ? SnapshotError::invalid_value : status;
    }
    std::int32_t interface_type_id = 0;
    status = invoke_slot(
        image,
        image_bytes,
        interface_type,
        type_slot::type_id,
        interface_type_id);
    if (status != SnapshotError::ok ||
        !hash.u32(static_cast<std::uint32_t>(interface_type_id))) {
      return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
    }
  }

  std::uint32_t factory_count = 0;
  std::uint32_t method_count = 0;
  std::uint32_t property_count = 0;
  std::uint32_t behaviour_count = 0;
#define GORE_AS_SNAPSHOT_INVOKE(slot, output, ...)                              \
  do {                                                                          \
    const auto status = invoke_slot(                                             \
        image, image_bytes, type, slot, output __VA_OPT__(, ) __VA_ARGS__);     \
    if (status != SnapshotError::ok) return status;                              \
  } while (false)
  GORE_AS_SNAPSHOT_INVOKE(type_slot::factory_count, factory_count);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::method_count, method_count);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::property_count, property_count);
  GORE_AS_SNAPSHOT_INVOKE(type_slot::behaviour_count, behaviour_count);
#undef GORE_AS_SNAPSHOT_INVOKE
  if (!add_count(counts.functions, factory_count) ||
      !add_count(counts.functions, method_count) ||
      !add_count(counts.functions, behaviour_count) ||
      !add_count(counts.object_properties, property_count) || !hash.u32(factory_count) ||
      !hash.u32(method_count) || !hash.u32(property_count) ||
      !hash.u32(behaviour_count)) {
    return SnapshotError::limit_exceeded;
  }
  for (std::uint32_t index = 0; index < factory_count; ++index) {
    std::uintptr_t function = 0;
    const auto status = invoke_slot(
        image, image_bytes, type, type_slot::factory_by_index, function, index);
    if (status != SnapshotError::ok) return status;
    const auto hash_status = hash_function(image, image_bytes, function, hash);
    if (hash_status != SnapshotError::ok) return hash_status;
  }
  for (std::uint32_t index = 0; index < method_count; ++index) {
    std::uintptr_t function = 0;
    const auto status = invoke_slot(
        image, image_bytes, type, type_slot::method_by_index, function, index);
    if (status != SnapshotError::ok) return status;
    const auto hash_status = hash_function(image, image_bytes, function, hash);
    if (hash_status != SnapshotError::ok) return hash_status;
  }
  for (std::uint32_t index = 0; index < behaviour_count; ++index) {
    std::uintptr_t function = 0;
    std::int32_t behaviour = 0;
    const auto status = invoke_slot(
        image,
        image_bytes,
        type,
        type_slot::behaviour_by_index,
        function,
        index,
        &behaviour);
    if (status != SnapshotError::ok || !hash.u32(static_cast<std::uint32_t>(behaviour))) {
      return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
    }
    const auto hash_status = hash_function(image, image_bytes, function, hash);
    if (hash_status != SnapshotError::ok) return hash_status;
  }
  for (std::uint32_t index = 0; index < property_count; ++index) {
    const char* declaration = nullptr;
    const auto status = invoke_slot(
        image,
        image_bytes,
        type,
        type_slot::property_declaration,
        declaration,
        index,
        true);
    if (status != SnapshotError::ok || !hash.string(declaration)) {
      return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
    }
    const char* property_name = nullptr;
    std::int32_t property_type = 0;
    bool is_private = false;
    bool is_protected = false;
    std::int32_t offset = 0;
    bool is_reference = false;
    std::uint32_t property_mask = 0;
    std::int32_t composite_offset = 0;
    bool composite_indirect = false;
    std::int32_t result = 0;
    const auto property_status = invoke_slot(
        image,
        image_bytes,
        type,
        type_slot::property,
        result,
        index,
        &property_name,
        &property_type,
        &is_private,
        &is_protected,
        &offset,
        &is_reference,
        &property_mask,
        &composite_offset,
        &composite_indirect);
    if (property_status != SnapshotError::ok || result < 0 || !hash.string(property_name) ||
        !hash.u32(static_cast<std::uint32_t>(property_type)) ||
        !hash.u32(static_cast<std::uint32_t>(offset)) ||
        !hash.u32(static_cast<std::uint32_t>(composite_offset)) ||
        !hash.u32(property_mask) ||
        !hash.u32((is_private ? 1u : 0u) | (is_protected ? 2u : 0u) |
                  (is_reference ? 4u : 0u) | (composite_indirect ? 8u : 0u))) {
      return property_status == SnapshotError::ok ? SnapshotError::hash_failure
                                                  : property_status;
    }
  }
  return SnapshotError::ok;
}

}  // namespace

SnapshotError capture_public_registry_snapshot_v23300(
    const std::uintptr_t primary_image,
    const std::uint32_t primary_image_bytes,
    const std::uintptr_t engine,
    PublicRegistrySnapshot& snapshot_out) noexcept {
  if (primary_image == 0 || primary_image_bytes == 0 || engine == 0) {
    return SnapshotError::invalid_argument;
  }
  try {
    PublicRegistrySnapshot snapshot{};
    Sha256 hash;
    constexpr std::array<std::byte, 41> domain{
        std::byte{'g'}, std::byte{'o'}, std::byte{'r'}, std::byte{'e'}, std::byte{'-'},
        std::byte{'a'}, std::byte{'s'}, std::byte{'-'}, std::byte{'p'}, std::byte{'u'},
        std::byte{'b'}, std::byte{'l'}, std::byte{'i'}, std::byte{'c'}, std::byte{'-'},
        std::byte{'r'}, std::byte{'e'}, std::byte{'g'}, std::byte{'i'}, std::byte{'s'},
        std::byte{'t'}, std::byte{'r'}, std::byte{'y'}, std::byte{'-'}, std::byte{'s'},
        std::byte{'n'}, std::byte{'a'}, std::byte{'p'}, std::byte{'s'}, std::byte{'h'},
        std::byte{'o'}, std::byte{'t'}, std::byte{'-'}, std::byte{'v'}, std::byte{'2'},
        std::byte{'3'}, std::byte{'3'}, std::byte{'0'}, std::byte{'0'}, std::byte{'\0'},
        std::byte{0}};
    if (!hash.append(domain.data(), domain.size())) return SnapshotError::hash_failure;

    std::uint32_t global_functions = 0;
    std::uint32_t global_properties = 0;
    std::uint32_t object_types = 0;
    std::uint32_t enums = 0;
    std::uint32_t funcdefs = 0;
    std::uint32_t typedefs = 0;
#define GORE_AS_ENGINE_INVOKE(slot, output, ...)                                  \
  do {                                                                            \
    const auto status = invoke_slot(                                               \
        primary_image, primary_image_bytes, engine, slot, output __VA_OPT__(, )   \
            __VA_ARGS__);                                                         \
    if (status != SnapshotError::ok) return status;                                \
  } while (false)
    GORE_AS_ENGINE_INVOKE(engine_slot::global_function_count, global_functions);
    GORE_AS_ENGINE_INVOKE(engine_slot::global_property_count, global_properties);
    GORE_AS_ENGINE_INVOKE(engine_slot::object_type_count, object_types);
    GORE_AS_ENGINE_INVOKE(engine_slot::enum_count, enums);
    GORE_AS_ENGINE_INVOKE(engine_slot::funcdef_count, funcdefs);
    GORE_AS_ENGINE_INVOKE(engine_slot::typedef_count, typedefs);
#undef GORE_AS_ENGINE_INVOKE
    if (global_functions > kMaxItems || global_properties > kMaxItems ||
        object_types > kMaxItems || enums > kMaxItems || funcdefs > kMaxItems ||
        typedefs > kMaxItems || !add_count(snapshot.counts.functions, global_functions) ||
        !add_count(snapshot.counts.global_properties, global_properties) ||
        !add_count(snapshot.counts.funcdefs, funcdefs) ||
        !add_count(snapshot.counts.typedefs, typedefs) ||
        !add_count(snapshot.counts.types, object_types) ||
        !add_count(snapshot.counts.types, enums) || !add_count(snapshot.counts.types, funcdefs) ||
        !add_count(snapshot.counts.types, typedefs) || !hash.u32(global_functions) ||
        !hash.u32(global_properties) || !hash.u32(object_types) || !hash.u32(enums) ||
        !hash.u32(funcdefs) || !hash.u32(typedefs)) {
      return SnapshotError::limit_exceeded;
    }

    for (std::uint32_t index = 0; index < global_functions; ++index) {
      std::uintptr_t function = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::global_function_by_index,
          function,
          index);
      if (status != SnapshotError::ok) return status;
      const auto hash_status =
          hash_function(primary_image, primary_image_bytes, function, hash);
      if (hash_status != SnapshotError::ok) return hash_status;
    }
    for (std::uint32_t index = 0; index < global_properties; ++index) {
      const char* name = nullptr;
      const char* name_space = nullptr;
      const char* config_group = nullptr;
      std::int32_t type_id = 0;
      bool is_const = false;
      void* storage_pointer = nullptr;
      std::uint32_t access_mask = 0;
      std::int32_t result = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::global_property_by_index,
          result,
          index,
          &name,
          &name_space,
          &type_id,
          &is_const,
          &config_group,
          &storage_pointer,
          &access_mask);
      (void)storage_pointer;
      if (status != SnapshotError::ok || result < 0 || !hash.string(name) ||
          !hash.string(name_space) || !hash.string(config_group) ||
          !hash.u32(static_cast<std::uint32_t>(type_id)) ||
          !hash.u32(is_const ? 1u : 0u) || !hash.u32(access_mask)) {
        return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
      }
    }
    for (std::uint32_t index = 0; index < object_types; ++index) {
      std::uintptr_t type = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::object_type_by_index,
          type,
          index);
      if (status != SnapshotError::ok) return status;
      const auto type_status =
          hash_type(primary_image, primary_image_bytes, type, snapshot.counts, hash);
      if (type_status != SnapshotError::ok) return type_status;
    }
    for (std::uint32_t index = 0; index < enums; ++index) {
      std::uintptr_t type = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::enum_by_index,
          type,
          index);
      if (status != SnapshotError::ok) return status;
      const auto type_status =
          hash_type(primary_image, primary_image_bytes, type, snapshot.counts, hash);
      if (type_status != SnapshotError::ok) return type_status;
      std::uint32_t value_count = 0;
      const auto count_status = invoke_slot(
          primary_image,
          primary_image_bytes,
          type,
          type_slot::enum_value_count,
          value_count);
      if (count_status != SnapshotError::ok ||
          !add_count(snapshot.counts.enum_values, value_count) || !hash.u32(value_count)) {
        return count_status == SnapshotError::ok ? SnapshotError::limit_exceeded : count_status;
      }
      for (std::uint32_t value_index = 0; value_index < value_count; ++value_index) {
        const char* name = nullptr;
        std::int32_t value = 0;
        const auto value_status = invoke_slot(
            primary_image,
            primary_image_bytes,
            type,
            type_slot::enum_value_by_index,
            name,
            value_index,
            &value);
        if (value_status != SnapshotError::ok || !hash.string(name) ||
            !hash.u32(static_cast<std::uint32_t>(value))) {
          return value_status == SnapshotError::ok ? SnapshotError::hash_failure
                                                   : value_status;
        }
      }
    }
    for (std::uint32_t index = 0; index < funcdefs; ++index) {
      std::uintptr_t type = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::funcdef_by_index,
          type,
          index);
      if (status != SnapshotError::ok) return status;
      const auto type_status =
          hash_type(primary_image, primary_image_bytes, type, snapshot.counts, hash);
      if (type_status != SnapshotError::ok) return type_status;
      std::uintptr_t signature = 0;
      const auto signature_status = invoke_slot(
          primary_image,
          primary_image_bytes,
          type,
          type_slot::funcdef_signature,
          signature);
      if (signature_status != SnapshotError::ok) return signature_status;
      const auto hash_status =
          hash_function(primary_image, primary_image_bytes, signature, hash);
      if (hash_status != SnapshotError::ok) return hash_status;
    }
    for (std::uint32_t index = 0; index < typedefs; ++index) {
      std::uintptr_t type = 0;
      const auto status = invoke_slot(
          primary_image,
          primary_image_bytes,
          engine,
          engine_slot::typedef_by_index,
          type,
          index);
      if (status != SnapshotError::ok) return status;
      const auto type_status =
          hash_type(primary_image, primary_image_bytes, type, snapshot.counts, hash);
      if (type_status != SnapshotError::ok) return type_status;
      std::int32_t target_type = 0;
      const auto target_status = invoke_slot(
          primary_image,
          primary_image_bytes,
          type,
          type_slot::typedef_type_id,
          target_type);
      if (target_status != SnapshotError::ok ||
          !hash.u32(static_cast<std::uint32_t>(target_type))) {
        return target_status == SnapshotError::ok ? SnapshotError::hash_failure : target_status;
      }
    }
    std::int32_t string_factory_type = -1;
    std::int32_t default_array_type = -1;
    std::uint32_t ignored_flags = 0;
    auto status = invoke_slot(
        primary_image,
        primary_image_bytes,
        engine,
        engine_slot::string_factory_return_type,
        string_factory_type,
        &ignored_flags);
    if (status != SnapshotError::ok) return status;
    status = invoke_slot(
        primary_image,
        primary_image_bytes,
        engine,
        engine_slot::default_array_type,
        default_array_type);
    if (status != SnapshotError::ok || !hash.u32(static_cast<std::uint32_t>(string_factory_type)) ||
        !hash.u32(static_cast<std::uint32_t>(default_array_type))) {
      return status == SnapshotError::ok ? SnapshotError::hash_failure : status;
    }

    const std::uint64_t total = static_cast<std::uint64_t>(snapshot.counts.types) +
                                snapshot.counts.functions +
                                snapshot.counts.object_properties +
                                snapshot.counts.global_properties +
                                snapshot.counts.enum_values +
                                (string_factory_type >= 0 ? 1u : 0u) +
                                (default_array_type >= 0 ? 1u : 0u);
    if (total > kMaxItems || total > std::numeric_limits<std::uint32_t>::max()) {
      return SnapshotError::limit_exceeded;
    }
    snapshot.counts.total_registrations = static_cast<std::uint32_t>(total);
    if (!hash.u32(snapshot.counts.total_registrations) ||
        !hash.finish(snapshot.canonical_sha256)) {
      return SnapshotError::hash_failure;
    }
    snapshot_out = snapshot;
    return SnapshotError::ok;
  } catch (...) {
    return SnapshotError::invalid_value;
  }
}

SnapshotError capture_native_class_capabilities_v23300(
    const std::uintptr_t primary_image,
    const std::uint32_t primary_image_bytes,
    const std::uintptr_t engine,
    std::vector<NativeClassCapability>& capabilities_out) noexcept {
  if (primary_image == 0 || primary_image_bytes == 0 || engine == 0) {
    return SnapshotError::invalid_argument;
  }
  try {
    std::uint32_t object_types = 0;
    auto status = invoke_slot(
        primary_image, primary_image_bytes, engine, engine_slot::object_type_count,
        object_types);
    if (status != SnapshotError::ok) return status;
    if (object_types > kMaxItems) return SnapshotError::limit_exceeded;

    std::vector<NativeClassCapability> capabilities;
    capabilities.reserve(object_types);
    for (std::uint32_t index = 0; index < object_types; ++index) {
      std::uintptr_t type = 0;
      status = invoke_slot(
          primary_image, primary_image_bytes, engine, engine_slot::object_type_by_index,
          type, index);
      if (status != SnapshotError::ok || type == 0) {
        return status != SnapshotError::ok ? status : SnapshotError::invalid_value;
      }
      const char* name = nullptr;
      const char* name_space = nullptr;
      std::uintptr_t user_data = 0;
      if ((status = invoke_slot(
               primary_image, primary_image_bytes, type, type_slot::name, name)) !=
              SnapshotError::ok ||
          (status = invoke_slot(
               primary_image, primary_image_bytes, type, type_slot::name_space,
               name_space)) != SnapshotError::ok ||
          (status = invoke_slot(
               primary_image, primary_image_bytes, type, type_slot::get_user_data,
               user_data)) != SnapshotError::ok) {
        return status;
      }
      if (user_data == 0) continue;
      NativeClassCapability capability{};
      if ((status = copy_public_string(name, capability.angelscript_type_name)) !=
              SnapshotError::ok ||
          (status = copy_public_string(name_space, capability.name_space)) !=
              SnapshotError::ok) {
        return status;
      }
      capability.user_data = user_data;
      capabilities.push_back(std::move(capability));
    }
    capabilities_out = std::move(capabilities);
    return SnapshotError::ok;
  } catch (...) {
    return SnapshotError::limit_exceeded;
  }
}

SnapshotError empty_public_registry_snapshot_v23300(
    PublicRegistrySnapshot& snapshot_out) noexcept {
  try {
    PublicRegistrySnapshot snapshot{};
    Sha256 hash;
    constexpr std::array<std::byte, 41> domain{
        std::byte{'g'}, std::byte{'o'}, std::byte{'r'}, std::byte{'e'}, std::byte{'-'},
        std::byte{'a'}, std::byte{'s'}, std::byte{'-'}, std::byte{'p'}, std::byte{'u'},
        std::byte{'b'}, std::byte{'l'}, std::byte{'i'}, std::byte{'c'}, std::byte{'-'},
        std::byte{'r'}, std::byte{'e'}, std::byte{'g'}, std::byte{'i'}, std::byte{'s'},
        std::byte{'t'}, std::byte{'r'}, std::byte{'y'}, std::byte{'-'}, std::byte{'s'},
        std::byte{'n'}, std::byte{'a'}, std::byte{'p'}, std::byte{'s'}, std::byte{'h'},
        std::byte{'o'}, std::byte{'t'}, std::byte{'-'}, std::byte{'v'}, std::byte{'2'},
        std::byte{'3'}, std::byte{'3'}, std::byte{'0'}, std::byte{'0'}, std::byte{'\0'},
        std::byte{0}};
    if (!hash.append(domain.data(), domain.size())) return SnapshotError::hash_failure;
    for (std::uint32_t index = 0; index < 6; ++index) {
      if (!hash.u32(0)) return SnapshotError::hash_failure;
    }
    if (!hash.u32(0xffff'ffffu) || !hash.u32(0xffff'ffffu) || !hash.u32(0) ||
        !hash.finish(snapshot.canonical_sha256)) {
      return SnapshotError::hash_failure;
    }
    snapshot_out = snapshot;
    return SnapshotError::ok;
  } catch (...) {
    return SnapshotError::hash_failure;
  }
}

SnapshotError advance_public_registry_witness_v1(
    const PublicRegistrySnapshot& previous,
    const RegistryCounts& projected_counts,
    const std::string_view canonical_delta_json,
    PublicRegistrySnapshot& snapshot_out) noexcept {
  if (canonical_delta_json.empty() ||
      canonical_delta_json.size() > std::numeric_limits<std::uint32_t>::max() ||
      std::all_of(
          previous.canonical_sha256.begin(), previous.canonical_sha256.end(),
          [](const std::byte value) { return value == std::byte{0}; })) {
    return SnapshotError::invalid_argument;
  }
  try {
    Sha256 hash;
    constexpr std::array<std::byte, 40> domain{
        std::byte{'g'}, std::byte{'o'}, std::byte{'r'}, std::byte{'e'}, std::byte{'-'},
        std::byte{'a'}, std::byte{'s'}, std::byte{'-'}, std::byte{'r'}, std::byte{'e'},
        std::byte{'g'}, std::byte{'i'}, std::byte{'s'}, std::byte{'t'}, std::byte{'r'},
        std::byte{'y'}, std::byte{'-'}, std::byte{'w'}, std::byte{'i'}, std::byte{'t'},
        std::byte{'n'}, std::byte{'e'}, std::byte{'s'}, std::byte{'s'}, std::byte{'-'},
        std::byte{'d'}, std::byte{'e'}, std::byte{'l'}, std::byte{'t'}, std::byte{'a'},
        std::byte{'-'}, std::byte{'v'}, std::byte{'1'}, std::byte{0}, std::byte{0},
        std::byte{0}, std::byte{0}, std::byte{0}, std::byte{0}, std::byte{0}};
    if (!hash.append(domain.data(), domain.size()) ||
        !hash.append(previous.canonical_sha256.data(),
                     previous.canonical_sha256.size())) {
      return SnapshotError::hash_failure;
    }
    for (const auto value : {
             projected_counts.types, projected_counts.functions,
             projected_counts.object_properties, projected_counts.global_properties,
             projected_counts.enum_values, projected_counts.funcdefs,
             projected_counts.typedefs, projected_counts.total_registrations}) {
      if (!hash.u32(value)) return SnapshotError::hash_failure;
    }
    if (!hash.u32(static_cast<std::uint32_t>(canonical_delta_json.size())) ||
        !hash.append(canonical_delta_json.data(), canonical_delta_json.size())) {
      return SnapshotError::hash_failure;
    }
    PublicRegistrySnapshot snapshot{};
    snapshot.counts = projected_counts;
    if (!hash.finish(snapshot.canonical_sha256)) return SnapshotError::hash_failure;
    snapshot_out = snapshot;
    return SnapshotError::ok;
  } catch (...) {
    return SnapshotError::hash_failure;
  }
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

enum class FakeTypeKind : std::uint32_t { object, enumeration, funcdef, type_alias };

struct FakeFunction final {
  std::uintptr_t* vtable{};
  std::int32_t id{};
  const char* name{};
  const char* declaration{};
};

struct FakeType final {
  std::uintptr_t* vtable{};
  std::uint32_t alignment{4};
  FakeTypeKind kind{};
  std::int32_t type_id{};
  const char* name{};
  std::int32_t property_offset{16};
  FakeType* base_type{};
  FakeType* interface_type{};
};

struct FakeEngine final {
  std::uintptr_t* vtable{};
  FakeFunction* global_function{};
  FakeFunction* factory{};
  FakeFunction* method{};
  FakeFunction* behaviour{};
  FakeFunction* funcdef_signature{};
  FakeType* object{};
  FakeType* enumeration{};
  FakeType* funcdef{};
  FakeType* type_alias{};
};

std::array<std::uintptr_t, 39> g_fake_engine_vtable{};
std::array<std::uintptr_t, 40> g_fake_type_vtable{};
std::array<std::uintptr_t, 17> g_fake_function_vtable{};
FakeFunction g_fake_global{};
FakeFunction g_fake_factory{};
FakeFunction g_fake_method{};
FakeFunction g_fake_behaviour{};
FakeFunction g_fake_funcdef_signature{};
FakeType g_fake_object{};
FakeType g_fake_enum{};
FakeType g_fake_funcdef{};
FakeType g_fake_typedef{};
FakeEngine g_fake_engine{};

template <typename Function>
std::uintptr_t function_address(const Function function) noexcept {
  static_assert(sizeof(function) == sizeof(std::uintptr_t));
  std::uintptr_t result = 0;
  std::memcpy(&result, &function, sizeof(result));
  return result;
}

std::uint32_t __fastcall fake_one_count(std::uintptr_t) noexcept { return 1; }

std::uintptr_t __fastcall fake_global_function(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto& engine = *reinterpret_cast<const FakeEngine*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(engine.global_function) : 0;
}

std::uintptr_t __fastcall fake_object_type(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto& engine = *reinterpret_cast<const FakeEngine*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(engine.object) : 0;
}

std::uintptr_t __fastcall fake_enum_type(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto& engine = *reinterpret_cast<const FakeEngine*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(engine.enumeration) : 0;
}

std::uintptr_t __fastcall fake_funcdef_type(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto& engine = *reinterpret_cast<const FakeEngine*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(engine.funcdef) : 0;
}

std::uintptr_t __fastcall fake_typedef_type(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto& engine = *reinterpret_cast<const FakeEngine*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(engine.type_alias) : 0;
}

std::int32_t __fastcall fake_global_property(
    std::uintptr_t,
    const std::uint32_t index,
    const char** const name,
    const char** const name_space,
    std::int32_t* const type_id,
    bool* const is_const,
    const char** const config_group,
    void** const storage,
    std::uint32_t* const access_mask) noexcept {
  if (index != 0 || name == nullptr || name_space == nullptr || type_id == nullptr ||
      is_const == nullptr || config_group == nullptr || storage == nullptr ||
      access_mask == nullptr) {
    return -1;
  }
  *name = "GlobalValue";
  *name_space = "Fixture";
  *type_id = 4;
  *is_const = true;
  *config_group = "fixture";
  *storage = reinterpret_cast<void*>(static_cast<std::uintptr_t>(0x1234));
  *access_mask = 0x55aa;
  return 0;
}

std::int32_t __fastcall fake_string_factory_type(
    std::uintptr_t,
    std::uint32_t* const flags) noexcept {
  if (flags != nullptr) *flags = 0;
  return 42;
}

std::int32_t __fastcall fake_default_array_type(std::uintptr_t) noexcept { return 43; }

const char* __fastcall fake_type_config_group(std::uintptr_t) noexcept { return "fixture"; }
std::uint32_t __fastcall fake_type_access_mask(std::uintptr_t) noexcept { return 0x1020; }

const char* __fastcall fake_type_name(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->name;
}

const char* __fastcall fake_namespace(std::uintptr_t) noexcept { return "Fixture"; }

std::uint32_t __fastcall fake_type_flags(const std::uintptr_t object) noexcept {
  const auto kind = reinterpret_cast<const FakeType*>(object)->kind;
  return 0x100u << static_cast<std::uint32_t>(kind);
}

std::uint32_t __fastcall fake_type_size(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->kind == FakeTypeKind::object ? 32u : 4u;
}

std::int32_t __fastcall fake_type_id(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->type_id;
}

std::uintptr_t __fastcall fake_base_type(const std::uintptr_t object) noexcept {
  return reinterpret_cast<std::uintptr_t>(
      reinterpret_cast<const FakeType*>(object)->base_type);
}

std::uint32_t __fastcall fake_interface_count(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->interface_type == nullptr ? 0u : 1u;
}

std::uintptr_t __fastcall fake_interface_type(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  const auto* const type = reinterpret_cast<const FakeType*>(object);
  return index == 0 ? reinterpret_cast<std::uintptr_t>(type->interface_type) : 0;
}

std::uint32_t __fastcall fake_object_member_count(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->kind == FakeTypeKind::object ? 1u : 0u;
}

std::uintptr_t __fastcall fake_factory(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  if (index != 0 || reinterpret_cast<const FakeType*>(object)->kind != FakeTypeKind::object) {
    return 0;
  }
  // Every fake object belongs to the single engine below. These pointers are initialized before
  // the snapshot and remain private test capabilities, never captured values.
  return reinterpret_cast<std::uintptr_t>(g_fake_engine.factory);
}

std::uintptr_t __fastcall fake_method(
    const std::uintptr_t object,
    const std::uint32_t index) noexcept {
  if (index != 0 || reinterpret_cast<const FakeType*>(object)->kind != FakeTypeKind::object) {
    return 0;
  }
  return reinterpret_cast<std::uintptr_t>(g_fake_engine.method);
}

std::uintptr_t __fastcall fake_behaviour(
    const std::uintptr_t object,
    const std::uint32_t index,
    std::int32_t* const behaviour) noexcept {
  if (index != 0 || behaviour == nullptr ||
      reinterpret_cast<const FakeType*>(object)->kind != FakeTypeKind::object) {
    return 0;
  }
  *behaviour = 4;
  return reinterpret_cast<std::uintptr_t>(g_fake_engine.behaviour);
}

const char* __fastcall fake_property_declaration(
    const std::uintptr_t object,
    const std::uint32_t index,
    bool) noexcept {
  return index == 0 &&
                 reinterpret_cast<const FakeType*>(object)->kind == FakeTypeKind::object
             ? "int Fixture::Object::Value"
             : nullptr;
}

std::int32_t __fastcall fake_property(
    const std::uintptr_t object,
    const std::uint32_t index,
    const char** const name,
    std::int32_t* const type_id,
    bool* const is_private,
    bool* const is_protected,
    std::int32_t* const offset,
    bool* const is_reference,
    std::uint32_t* const access_mask,
    std::int32_t* const composite_offset,
    bool* const composite_indirect) noexcept {
  const auto& type = *reinterpret_cast<const FakeType*>(object);
  if (type.kind != FakeTypeKind::object || index != 0 || name == nullptr ||
      type_id == nullptr || is_private == nullptr || is_protected == nullptr ||
      offset == nullptr || is_reference == nullptr || access_mask == nullptr ||
      composite_offset == nullptr || composite_indirect == nullptr) {
    return -1;
  }
  *name = "Value";
  *type_id = 4;
  *is_private = false;
  *is_protected = true;
  *offset = type.property_offset;
  *is_reference = false;
  *access_mask = 0x77;
  *composite_offset = 0;
  *composite_indirect = false;
  return 0;
}

std::uint32_t __fastcall fake_enum_value_count(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->kind == FakeTypeKind::enumeration ? 1u : 0u;
}

const char* __fastcall fake_enum_value(
    const std::uintptr_t object,
    const std::uint32_t index,
    std::int32_t* const value) noexcept {
  if (index != 0 || value == nullptr ||
      reinterpret_cast<const FakeType*>(object)->kind != FakeTypeKind::enumeration) {
    return nullptr;
  }
  *value = 7;
  return "Seven";
}

std::int32_t __fastcall fake_typedef_target(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeType*>(object)->kind == FakeTypeKind::type_alias ? 4 : -1;
}

std::uintptr_t __fastcall fake_funcdef_signature(const std::uintptr_t object) noexcept {
  if (reinterpret_cast<const FakeType*>(object)->kind != FakeTypeKind::funcdef) return 0;
  return reinterpret_cast<std::uintptr_t>(g_fake_engine.funcdef_signature);
}

std::int32_t __fastcall fake_function_id(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeFunction*>(object)->id;
}

const char* __fastcall fake_function_config_group(std::uintptr_t) noexcept { return "fixture"; }
std::uint32_t __fastcall fake_function_access_mask(std::uintptr_t) noexcept { return 0x2040; }

const char* __fastcall fake_function_name(const std::uintptr_t object) noexcept {
  return reinterpret_cast<const FakeFunction*>(object)->name;
}

const char* __fastcall fake_function_declaration(
    const std::uintptr_t object,
    bool,
    bool,
    bool,
    bool) noexcept {
  return reinterpret_cast<const FakeFunction*>(object)->declaration;
}

void initialize_fake_registry() noexcept {
  g_fake_engine_vtable.fill(0);
  g_fake_type_vtable.fill(0);
  g_fake_function_vtable.fill(0);

  g_fake_engine_vtable[engine_slot::global_function_count] =
      function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::global_function_by_index] =
      function_address(&fake_global_function);
  g_fake_engine_vtable[engine_slot::global_property_count] =
      function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::global_property_by_index] =
      function_address(&fake_global_property);
  g_fake_engine_vtable[engine_slot::object_type_count] = function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::object_type_by_index] = function_address(&fake_object_type);
  g_fake_engine_vtable[engine_slot::string_factory_return_type] =
      function_address(&fake_string_factory_type);
  g_fake_engine_vtable[engine_slot::default_array_type] =
      function_address(&fake_default_array_type);
  g_fake_engine_vtable[engine_slot::enum_count] = function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::enum_by_index] = function_address(&fake_enum_type);
  g_fake_engine_vtable[engine_slot::funcdef_count] = function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::funcdef_by_index] = function_address(&fake_funcdef_type);
  g_fake_engine_vtable[engine_slot::typedef_count] = function_address(&fake_one_count);
  g_fake_engine_vtable[engine_slot::typedef_by_index] = function_address(&fake_typedef_type);

  g_fake_type_vtable[type_slot::config_group] = function_address(&fake_type_config_group);
  g_fake_type_vtable[type_slot::access_mask] = function_address(&fake_type_access_mask);
  g_fake_type_vtable[type_slot::name] = function_address(&fake_type_name);
  g_fake_type_vtable[type_slot::name_space] = function_address(&fake_namespace);
  g_fake_type_vtable[type_slot::base_type] = function_address(&fake_base_type);
  g_fake_type_vtable[type_slot::flags] = function_address(&fake_type_flags);
  g_fake_type_vtable[type_slot::size] = function_address(&fake_type_size);
  g_fake_type_vtable[type_slot::type_id] = function_address(&fake_type_id);
  g_fake_type_vtable[type_slot::interface_count] =
      function_address(&fake_interface_count);
  g_fake_type_vtable[type_slot::interface_by_index] =
      function_address(&fake_interface_type);
  g_fake_type_vtable[type_slot::factory_count] = function_address(&fake_object_member_count);
  g_fake_type_vtable[type_slot::factory_by_index] = function_address(&fake_factory);
  g_fake_type_vtable[type_slot::method_count] = function_address(&fake_object_member_count);
  g_fake_type_vtable[type_slot::method_by_index] = function_address(&fake_method);
  g_fake_type_vtable[type_slot::property_count] = function_address(&fake_object_member_count);
  g_fake_type_vtable[type_slot::property] = function_address(&fake_property);
  g_fake_type_vtable[type_slot::property_declaration] =
      function_address(&fake_property_declaration);
  g_fake_type_vtable[type_slot::behaviour_count] = function_address(&fake_object_member_count);
  g_fake_type_vtable[type_slot::behaviour_by_index] = function_address(&fake_behaviour);
  g_fake_type_vtable[type_slot::enum_value_count] = function_address(&fake_enum_value_count);
  g_fake_type_vtable[type_slot::enum_value_by_index] = function_address(&fake_enum_value);
  g_fake_type_vtable[type_slot::typedef_type_id] = function_address(&fake_typedef_target);
  g_fake_type_vtable[type_slot::funcdef_signature] = function_address(&fake_funcdef_signature);

  g_fake_function_vtable[function_slot::id] = function_address(&fake_function_id);
  g_fake_function_vtable[function_slot::config_group] =
      function_address(&fake_function_config_group);
  g_fake_function_vtable[function_slot::access_mask] =
      function_address(&fake_function_access_mask);
  g_fake_function_vtable[function_slot::name] = function_address(&fake_function_name);
  g_fake_function_vtable[function_slot::name_space] = function_address(&fake_namespace);
  g_fake_function_vtable[function_slot::declaration] =
      function_address(&fake_function_declaration);

  g_fake_global = {g_fake_function_vtable.data(), 10, "GlobalFn", "int Fixture::GlobalFn()"};
  g_fake_factory = {g_fake_function_vtable.data(), 11, "Factory", "Fixture::Object@ f()"};
  g_fake_method = {g_fake_function_vtable.data(), 12, "Method", "void Fixture::Object::Method()"};
  g_fake_behaviour =
      {g_fake_function_vtable.data(), 13, "Construct", "void f()"};
  g_fake_funcdef_signature =
      {g_fake_function_vtable.data(), 14, "Callback", "void Fixture::Callback(int)"};
  g_fake_object = {
      g_fake_type_vtable.data(), 8, FakeTypeKind::object, 100, "Object", 16};
  g_fake_enum = {
      g_fake_type_vtable.data(), 4, FakeTypeKind::enumeration, 101, "Enum", 0};
  g_fake_funcdef =
      {g_fake_type_vtable.data(), 8, FakeTypeKind::funcdef, 102, "Callback", 0};
  g_fake_typedef =
      {g_fake_type_vtable.data(), 4, FakeTypeKind::type_alias, 103, "Alias", 0};
  g_fake_object.base_type = &g_fake_typedef;
  g_fake_object.interface_type = &g_fake_funcdef;
  g_fake_engine = {g_fake_engine_vtable.data(),
                   &g_fake_global,
                   &g_fake_factory,
                   &g_fake_method,
                   &g_fake_behaviour,
                   &g_fake_funcdef_signature,
                   &g_fake_object,
                   &g_fake_enum,
                   &g_fake_funcdef,
                   &g_fake_typedef};
}

bool fixture_image(std::uintptr_t& image, std::uint32_t& image_bytes) noexcept {
  MEMORY_BASIC_INFORMATION memory{};
  if (VirtualQuery(g_fake_engine_vtable.data(), &memory, sizeof(memory)) != sizeof(memory) ||
      memory.AllocationBase == nullptr) {
    return false;
  }
  image = reinterpret_cast<std::uintptr_t>(memory.AllocationBase);
  if (!readable_range(image, sizeof(IMAGE_DOS_HEADER))) return false;
  const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(image);
  if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 ||
      !readable_range(image + static_cast<std::uint32_t>(dos->e_lfanew),
                      sizeof(IMAGE_NT_HEADERS64))) {
    return false;
  }
  const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS64*>(
      image + static_cast<std::uint32_t>(dos->e_lfanew));
  if (nt->Signature != IMAGE_NT_SIGNATURE ||
      nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC ||
      nt->OptionalHeader.SizeOfImage == 0) {
    return false;
  }
  image_bytes = nt->OptionalHeader.SizeOfImage;
  return true;
}

}  // namespace

bool public_registry_snapshot_selftest_v23300() noexcept {
  initialize_fake_registry();
  std::uintptr_t image = 0;
  std::uint32_t image_bytes = 0;
  if (!fixture_image(image, image_bytes)) return false;

  PublicRegistrySnapshot first{};
  PublicRegistrySnapshot second{};
  if (capture_public_registry_snapshot_v23300(
          image,
          image_bytes,
          reinterpret_cast<std::uintptr_t>(&g_fake_engine),
          first) != SnapshotError::ok ||
      capture_public_registry_snapshot_v23300(
          image,
          image_bytes,
          reinterpret_cast<std::uintptr_t>(&g_fake_engine),
          second) != SnapshotError::ok ||
      first.counts.types != 4 || first.counts.functions != 4 ||
      first.counts.object_properties != 1 || first.counts.global_properties != 1 ||
      first.counts.enum_values != 1 || first.counts.funcdefs != 1 ||
      first.counts.typedefs != 1 || first.counts.total_registrations != 13 ||
      first.canonical_sha256 != second.canonical_sha256) {
    return false;
  }

  g_fake_object.property_offset = 24;
  PublicRegistrySnapshot changed{};
  const auto changed_status = capture_public_registry_snapshot_v23300(
      image,
      image_bytes,
      reinterpret_cast<std::uintptr_t>(&g_fake_engine),
      changed);
  g_fake_object.property_offset = 16;
  if (changed_status != SnapshotError::ok ||
      changed.canonical_sha256 == first.canonical_sha256) {
    return false;
  }

  const auto valid_target = g_fake_engine_vtable[engine_slot::global_function_count];
  g_fake_engine_vtable[engine_slot::global_function_count] =
      function_address(&GetCurrentProcessId);
  PublicRegistrySnapshot rejected{};
  const auto rejected_status = capture_public_registry_snapshot_v23300(
      image,
      image_bytes,
      reinterpret_cast<std::uintptr_t>(&g_fake_engine),
      rejected);
  g_fake_engine_vtable[engine_slot::global_function_count] = valid_target;
  return rejected_status == SnapshotError::abi_target_outside_image;
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
