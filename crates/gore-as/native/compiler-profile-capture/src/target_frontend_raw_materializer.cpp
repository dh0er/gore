#include "target_frontend_raw_materializer.hpp"

#include "gore_as_capture/format.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <cstring>
#include <limits>
#include <set>
#include <string_view>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

using Error = TargetFrontendRawError;
using RegionKind = TargetRawRegionKind;

// BuildID 24539464 contains 12,904 Blueprint push-argument specializations.
// Their independently allocated FString payloads legitimately produce more
// than 4,096 immutable extents. The builder has the same 16,384-claim bound;
// the separate 128-MiB byte cap remains the stronger memory limit.
constexpr std::size_t kMaximumSnapshotRegions = 16'384;
constexpr std::size_t kMaximumSnapshotBytes = 128u * 1024u * 1024u;
constexpr std::size_t kMaximumTextCodeUnits = 8u * 1024u * 1024u;
constexpr std::size_t kMaximumTextBytes = 16u * 1024u * 1024u;
constexpr std::size_t kMaximumContainerItems = 1'000'000;
constexpr std::size_t kMaximumFiles = 65'536;
constexpr std::size_t kMaximumChunks = 1'000'000;
constexpr std::size_t kMaximumObjectDepth = 64;
constexpr std::int32_t kMaximumSharedReferences = 0x3fff'ffff;
constexpr std::uint32_t kFNamePoolRva = 0x09af8600;
constexpr std::size_t kFNamePoolBlocks = 0x10;
constexpr std::uint32_t kFNameBlockShift = 16;
constexpr std::uint32_t kFNameOffsetMask = 0xffff;
constexpr std::uint32_t kFNameEntryStride = 2;
constexpr std::uint32_t kFNameHeaderLengthShift = 6;
constexpr std::uint16_t kFNameWideMask = 1;
constexpr std::uint32_t kMaximumFNameBlocks = 1u << 13;
constexpr std::size_t kGraphDelegateBytes = 0x18;
constexpr std::size_t kGraphDelegateInvocationList = 0x00;
constexpr std::size_t kGraphDelegateNum = 0x08;
constexpr std::size_t kGraphDelegateMax = 0x0c;
constexpr std::size_t kGraphDelegateCompactionThreshold = 0x10;
constexpr std::size_t kGraphDelegateBroadcastCount = 0x14;
constexpr std::size_t kSharedControllerVtable = 0x00;
constexpr std::size_t kSharedControllerStrong = 0x08;
constexpr std::size_t kSharedControllerWeak = 0x0c;
// UE 5.4's FDefaultBitArrayAllocator is TInlineAllocator<4>: four inline uint32
// words followed by the secondary allocation pointer, then NumBits/MaxBits.
// The free-list fields follow the complete TBitArray rather than its inline
// storage. These offsets were also verified against BuildID 24539464.
constexpr std::size_t kSparseInlineAllocationFlags = 0x10;
constexpr std::size_t kSparseSecondaryAllocation = 0x20;
constexpr std::size_t kSparseAllocationNum = 0x28;
constexpr std::size_t kSparseAllocationMax = 0x2c;
constexpr std::size_t kSparseFirstFree = 0x30;
constexpr std::size_t kSparseNumFree = 0x34;
constexpr std::size_t kSparseInlineAllocationWords = 4;
constexpr std::size_t kMapFlagElementStride = 0x20;
constexpr std::size_t kMapFlagValue = 0x10;
constexpr std::size_t kSetStringElementStride = 0x18;
constexpr std::size_t kFileStride = 0xc8;
constexpr std::size_t kFileModule = 0x00;
constexpr std::size_t kFileAbsolute = 0x20;
constexpr std::size_t kFileRelative = 0x30;
constexpr std::size_t kFileRawCode = 0x40;
constexpr std::size_t kFileChunkBlocks = 0x50;
constexpr std::size_t kFileChunkCount = 0x60;
constexpr std::size_t kFileProcessedCode = 0x68;
constexpr std::size_t kFileGeneratedCode = 0x78;
constexpr std::size_t kFileImportsResolved = 0xa8;
constexpr std::size_t kFileResolvingImports = 0xa9;
constexpr std::size_t kFileLoadAsynchronous = 0xaa;
constexpr std::size_t kFileAsyncReadHandle = 0xb0;
constexpr std::size_t kFileAsyncSizeRequest = 0xb8;
constexpr std::size_t kFileAsyncReadRequest = 0xc0;
constexpr std::size_t kChunkStride = 0x90;
constexpr std::size_t kChunkElementsPerBlock = 0x71;
constexpr std::size_t kChunkType = 0x00;
constexpr std::size_t kChunkContent = 0x08;
constexpr std::size_t kChunkComment = 0x18;
constexpr std::size_t kChunkClassDescriptor = 0x58;
constexpr std::size_t kChunkNamespace = 0x68;
constexpr std::size_t kChunkNamespaceSet = 0x78;
constexpr std::size_t kChunkFileLine = 0x80;
constexpr std::size_t kChunkStart = 0x84;
constexpr std::size_t kChunkEnd = 0x88;
constexpr std::size_t kModuleName = 0x00;
constexpr std::size_t kModuleCode = 0x10;
constexpr std::size_t kModuleCodeHash = 0x20;
constexpr std::size_t kCodeSectionStride = 0x38;
constexpr std::size_t kCodeSectionRelative = 0x00;
constexpr std::size_t kCodeSectionAbsolute = 0x10;
constexpr std::size_t kCodeSectionCode = 0x20;
constexpr std::size_t kCodeSectionHash = 0x30;
constexpr std::size_t kClassName = 0x00;
constexpr std::size_t kClassSuperName = 0x10;
constexpr std::size_t kClassCodeSuper = 0x20;
constexpr std::size_t kClassSuperIsCode = 0x28;
constexpr std::size_t kClassComposeOnto = 0xf8;
constexpr std::size_t kClassNamespace = 0x108;
constexpr std::size_t kClassNamespaceSet = 0x118;
constexpr std::size_t kUObjectVtable = 0x00;
constexpr std::size_t kUObjectFlags = 0x08;
constexpr std::size_t kUObjectInternalIndex = 0x0c;
constexpr std::size_t kUObjectClass = 0x10;
constexpr std::size_t kUObjectName = 0x18;
constexpr std::size_t kUObjectOuter = 0x20;
constexpr std::size_t kUStructSuper = 0x40;
constexpr std::size_t kUStructPropertiesSize = 0x58;

struct RawArray final {
  std::uintptr_t data{};
  std::int32_t num{};
  std::int32_t capacity{};
};
static_assert(sizeof(RawArray) == 0x10);

struct RawShared final {
  std::uintptr_t object{};
  std::uintptr_t controller{};
};
static_assert(sizeof(RawShared) == 0x10);

template <typename Type>
Error read_value(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    const RegionKind kind,
    Type& output) noexcept {
  static_assert(std::is_trivially_copyable_v<Type>);
  return snapshot.read(
      address,
      {reinterpret_cast<std::byte*>(&output), sizeof(output)},
      kind);
}

template <typename Type>
Error read_any_value(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    Type& output) noexcept {
  static_assert(std::is_trivially_copyable_v<Type>);
  return snapshot.read_any(address, {reinterpret_cast<std::byte*>(&output), sizeof(output)});
}

bool add_address(
    const std::uintptr_t base,
    const std::size_t offset,
    std::uintptr_t& output) noexcept {
  if (base > std::numeric_limits<std::uintptr_t>::max() - offset) return false;
  output = base + offset;
  return true;
}

bool multiply_size(
    const std::size_t left,
    const std::size_t right,
    std::size_t& output) noexcept {
  if (left != 0 && right > std::numeric_limits<std::size_t>::max() / left) return false;
  output = left * right;
  return true;
}

Error read_array(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    const std::size_t stride,
    const std::size_t maximum_items,
    RawArray& array) noexcept {
  const auto status = read_any_value(snapshot, address, array);
  if (status != Error::ok) return status;
  if (array.num < 0 || array.capacity < 0 || array.num > array.capacity ||
      static_cast<std::size_t>(array.num) > maximum_items ||
      static_cast<std::size_t>(array.capacity) > maximum_items) {
    return Error::invalid_container;
  }
  if (array.capacity == 0) {
    return array.num == 0 && array.data == 0 ? Error::ok : Error::invalid_container;
  }
  if (array.data == 0 || stride == 0) return Error::invalid_container;
  std::size_t bytes = 0;
  if (!multiply_size(static_cast<std::size_t>(array.capacity), stride, bytes) ||
      !snapshot.is_data_address(array.data, bytes)) {
    return Error::invalid_container;
  }
  return Error::ok;
}

Error utf16_to_utf8(const std::span<const std::uint16_t> input, std::string& output) {
  output.clear();
  if (input.size() > kMaximumTextCodeUnits) return Error::limit_exceeded;
  output.reserve(std::min(kMaximumTextBytes, input.size() * 3));
  for (std::size_t index = 0; index < input.size(); ++index) {
    std::uint32_t scalar = input[index];
    if (scalar >= 0xd800 && scalar <= 0xdbff) {
      if (++index == input.size()) return Error::invalid_utf16;
      const std::uint32_t low = input[index];
      if (low < 0xdc00 || low > 0xdfff) return Error::invalid_utf16;
      scalar = 0x10000 + ((scalar - 0xd800) << 10) + (low - 0xdc00);
    } else if (scalar >= 0xdc00 && scalar <= 0xdfff) {
      return Error::invalid_utf16;
    }
    if (scalar == 0 || scalar > 0x10ffff) return Error::invalid_utf16;
    if (scalar <= 0x7f) {
      output.push_back(static_cast<char>(scalar));
    } else if (scalar <= 0x7ff) {
      output.push_back(static_cast<char>(0xc0 | (scalar >> 6)));
      output.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    } else if (scalar <= 0xffff) {
      output.push_back(static_cast<char>(0xe0 | (scalar >> 12)));
      output.push_back(static_cast<char>(0x80 | ((scalar >> 6) & 0x3f)));
      output.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    } else {
      output.push_back(static_cast<char>(0xf0 | (scalar >> 18)));
      output.push_back(static_cast<char>(0x80 | ((scalar >> 12) & 0x3f)));
      output.push_back(static_cast<char>(0x80 | ((scalar >> 6) & 0x3f)));
      output.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    }
    if (output.size() > kMaximumTextBytes) return Error::limit_exceeded;
  }
  return Error::ok;
}

Error read_fstring(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    std::string& output,
    std::size_t* const code_units = nullptr) {
  RawArray array{};
  const auto status = read_array(snapshot, address, sizeof(std::uint16_t),
                                 kMaximumTextCodeUnits + 1, array);
  if (status != Error::ok) return status;
  if (array.num == 0) {
    output.clear();
    if (code_units != nullptr) *code_units = 0;
    return Error::ok;
  }
  std::vector<std::uint16_t> characters(static_cast<std::size_t>(array.num));
  const auto read_status = snapshot.read(
      array.data,
      {reinterpret_cast<std::byte*>(characters.data()),
       characters.size() * sizeof(std::uint16_t)},
      RegionKind::immutable_data);
  if (read_status != Error::ok) return read_status;
  if (characters.back() != 0 ||
      std::find(characters.begin(), characters.end() - 1, std::uint16_t{0}) !=
          characters.end() - 1) {
    return Error::invalid_utf16;
  }
  characters.pop_back();
  if (code_units != nullptr) *code_units = characters.size();
  return utf16_to_utf8(characters, output);
}

Error read_raw_fstring_units(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    std::vector<std::uint16_t>& output) {
  RawArray array{};
  const auto status = read_array(snapshot, address, sizeof(std::uint16_t),
                                 kMaximumTextCodeUnits + 1, array);
  if (status != Error::ok) return status;
  output.clear();
  if (array.num == 0) return Error::ok;
  output.resize(static_cast<std::size_t>(array.num));
  const auto read_status = snapshot.read(
      array.data,
      {reinterpret_cast<std::byte*>(output.data()), output.size() * sizeof(std::uint16_t)},
      RegionKind::immutable_data);
  if (read_status != Error::ok) return read_status;
  if (output.back() != 0 ||
      std::find(output.begin(), output.end() - 1, std::uint16_t{0}) != output.end() - 1) {
    return Error::invalid_utf16;
  }
  output.pop_back();
  return Error::ok;
}

bool valid_plain_text(const std::string_view value, const bool allow_empty) noexcept {
  if ((!allow_empty && value.empty()) || value.size() > kMaximumTextBytes ||
      value.find('\0') != std::string_view::npos) {
    return false;
  }
  return true;
}

Error read_bool(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    bool& output) noexcept {
  std::uint8_t raw = 0;
  const auto status = read_any_value(snapshot, address, raw);
  if (status != Error::ok) return status;
  if (raw > 1) return Error::target_layout_drift;
  output = raw != 0;
  return Error::ok;
}

Error read_shared(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    const bool nullable,
    RawShared& shared) noexcept {
  const auto status = read_any_value(snapshot, address, shared);
  if (status != Error::ok) return status;
  if (shared.object == 0 || shared.controller == 0) {
    if (nullable && shared.object == 0 && shared.controller == 0) return Error::ok;
    return Error::invalid_shared_owner;
  }
  if (!snapshot.is_data_address(shared.object) ||
      !snapshot.is_data_address(shared.controller, 0x10)) {
    return Error::invalid_shared_owner;
  }
  std::uintptr_t vtable = 0;
  std::int32_t strong = 0;
  std::int32_t weak = 0;
  if (read_value(snapshot, shared.controller + kSharedControllerVtable,
                 RegionKind::immutable_data, vtable) != Error::ok ||
      read_value(snapshot, shared.controller + kSharedControllerStrong,
                 RegionKind::immutable_data, strong) != Error::ok ||
      read_value(snapshot, shared.controller + kSharedControllerWeak,
                 RegionKind::immutable_data, weak) != Error::ok ||
      !snapshot.is_image_address(vtable) || strong <= 0 || weak <= 0 ||
      strong > kMaximumSharedReferences || weak > kMaximumSharedReferences) {
    return Error::invalid_shared_owner;
  }
  return Error::ok;
}

Error read_sparse_slots(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t address,
    const std::size_t stride,
    const std::size_t maximum_items,
    std::vector<std::uintptr_t>& slots) {
  RawArray elements{};
  auto status = read_array(snapshot, address, stride, maximum_items, elements);
  if (status != Error::ok) return status;
  std::uintptr_t secondary_allocation = 0;
  std::int32_t allocation_num = 0;
  std::int32_t allocation_capacity = 0;
  if (read_any_value(snapshot, address + kSparseSecondaryAllocation,
                     secondary_allocation) != Error::ok ||
      read_any_value(snapshot, address + kSparseAllocationNum, allocation_num) !=
          Error::ok ||
      read_any_value(snapshot, address + kSparseAllocationMax,
                     allocation_capacity) != Error::ok ||
      allocation_num < 0 || allocation_capacity < 0 ||
      allocation_num > allocation_capacity || allocation_num != elements.num ||
      static_cast<std::size_t>(allocation_capacity) > maximum_items) {
    return Error::invalid_container;
  }
  std::int32_t first_free = 0;
  std::int32_t num_free = 0;
  if (read_any_value(snapshot, address + kSparseFirstFree, first_free) != Error::ok ||
      read_any_value(snapshot, address + kSparseNumFree, num_free) != Error::ok ||
      num_free < 0 || num_free > elements.num ||
      (num_free == 0 ? first_free != -1
                     : (first_free < 0 || first_free >= elements.num))) {
    return Error::invalid_container;
  }
  if (elements.num == 0) {
    if (allocation_num != 0 || num_free != 0 || allocation_capacity != 0 ||
        secondary_allocation != 0) {
      return Error::invalid_container;
    }
    slots.clear();
    return Error::ok;
  }
  const std::size_t words = (static_cast<std::size_t>(elements.num) + 31) / 32;
  const std::size_t capacity_words =
      (static_cast<std::size_t>(allocation_capacity) + 31) / 32;
  const auto allocation_data =
      capacity_words <= kSparseInlineAllocationWords
          ? address + kSparseInlineAllocationFlags
          : secondary_allocation;
  if ((capacity_words <= kSparseInlineAllocationWords && secondary_allocation != 0) ||
      (capacity_words > kSparseInlineAllocationWords && secondary_allocation == 0) ||
      !snapshot.is_data_address(allocation_data,
                                capacity_words * sizeof(std::uint32_t))) {
    return Error::invalid_container;
  }
  std::vector<std::uint32_t> bits(words);
  status = snapshot.read(
      allocation_data,
      {reinterpret_cast<std::byte*>(bits.data()), bits.size() * sizeof(bits.front())},
      RegionKind::immutable_data);
  if (status != Error::ok) return status;
  const auto tail_bits = static_cast<std::uint32_t>(elements.num) & 31u;
  if (tail_bits != 0 && (bits.back() & ~((1u << tail_bits) - 1u)) != 0) {
    return Error::invalid_container;
  }
  slots.clear();
  slots.reserve(static_cast<std::size_t>(elements.num - num_free));
  for (std::int32_t index = 0; index < elements.num; ++index) {
    if ((bits[static_cast<std::size_t>(index) / 32] &
         (1u << (static_cast<std::uint32_t>(index) & 31))) == 0) {
      continue;
    }
    std::size_t offset = 0;
    if (!multiply_size(static_cast<std::size_t>(index), stride, offset) ||
        elements.data > std::numeric_limits<std::uintptr_t>::max() - offset) {
      return Error::address_overflow;
    }
    slots.push_back(elements.data + offset);
  }
  if (slots.size() != static_cast<std::size_t>(elements.num - num_free)) {
    return Error::invalid_container;
  }
  return Error::ok;
}

constexpr std::uint64_t rotate_left(const std::uint64_t value, const int bits) noexcept {
  return std::rotl(value, bits);
}

std::uint64_t read_u64_le(const std::byte* bytes) noexcept {
  std::uint64_t value = 0;
  std::memcpy(&value, bytes, sizeof(value));
  return value;
}

std::uint32_t read_u32_le(const std::byte* bytes) noexcept {
  std::uint32_t value = 0;
  std::memcpy(&value, bytes, sizeof(value));
  return value;
}

std::uint64_t xxh64(const std::span<const std::byte> bytes) noexcept {
  constexpr std::uint64_t prime1 = 11400714785074694791ull;
  constexpr std::uint64_t prime2 = 14029467366897019727ull;
  constexpr std::uint64_t prime3 = 1609587929392839161ull;
  constexpr std::uint64_t prime4 = 9650029242287828579ull;
  constexpr std::uint64_t prime5 = 2870177450012600261ull;
  const auto round = [](std::uint64_t accumulator, const std::uint64_t input) noexcept {
    accumulator += input * prime2;
    accumulator = rotate_left(accumulator, 31);
    return accumulator * prime1;
  };
  const std::byte empty{};
  const std::byte* cursor = bytes.empty() ? &empty : bytes.data();
  const std::byte* const end = cursor + bytes.size();
  std::uint64_t hash = 0;
  if (bytes.size() >= 32) {
    std::uint64_t v1 = prime1 + prime2;
    std::uint64_t v2 = prime2;
    std::uint64_t v3 = 0;
    std::uint64_t v4 = 0 - prime1;
    const std::byte* const limit = end - 32;
    do {
      v1 = round(v1, read_u64_le(cursor));
      cursor += 8;
      v2 = round(v2, read_u64_le(cursor));
      cursor += 8;
      v3 = round(v3, read_u64_le(cursor));
      cursor += 8;
      v4 = round(v4, read_u64_le(cursor));
      cursor += 8;
    } while (cursor <= limit);
    hash = rotate_left(v1, 1) + rotate_left(v2, 7) + rotate_left(v3, 12) +
           rotate_left(v4, 18);
    for (const auto value : {v1, v2, v3, v4}) {
      const auto mixed = round(0, value);
      hash ^= mixed;
      hash = hash * prime1 + prime4;
    }
  } else {
    hash = prime5;
  }
  hash += bytes.size();
  while (cursor + 8 <= end) {
    const auto mixed = round(0, read_u64_le(cursor));
    hash ^= mixed;
    hash = rotate_left(hash, 27) * prime1 + prime4;
    cursor += 8;
  }
  if (cursor + 4 <= end) {
    hash ^= static_cast<std::uint64_t>(read_u32_le(cursor)) * prime1;
    hash = rotate_left(hash, 23) * prime2 + prime3;
    cursor += 4;
  }
  while (cursor < end) {
    hash ^= std::to_integer<std::uint8_t>(*cursor) * prime5;
    hash = rotate_left(hash, 11) * prime1;
    ++cursor;
  }
  hash ^= hash >> 33;
  hash *= prime2;
  hash ^= hash >> 29;
  hash *= prime3;
  hash ^= hash >> 32;
  return hash;
}

Error materialize_module_name(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t module,
    std::string& name) {
  if (!snapshot.is_data_address(module, kModuleCodeHash + sizeof(std::int64_t))) {
    return Error::unreadable_address;
  }
  const auto status = read_fstring(snapshot, module + kModuleName, name);
  return status == Error::ok && valid_plain_text(name, false) ? Error::ok
                                                              : Error::invalid_utf8;
}

Error materialize_class_name(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t descriptor,
    std::string& class_name) {
  if (!snapshot.is_data_address(descriptor, kClassNamespaceSet + 1)) {
    return Error::unreadable_address;
  }
  const auto status = read_fstring(snapshot, descriptor + kClassName, class_name);
  return status == Error::ok && valid_plain_text(class_name, false) ? Error::ok
                                                                    : Error::invalid_utf8;
}

Error read_chunk(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t chunk,
    const std::size_t raw_code_units,
    TargetFrontendRawChunk& output,
    std::uintptr_t* const class_object = nullptr) {
  std::uint8_t type = 0;
  if (read_value(snapshot, chunk + kChunkType, RegionKind::immutable_data, type) != Error::ok ||
      type > 3) {
    return Error::target_layout_drift;
  }
  TargetFrontendRawChunk value{};
  value.type = type;
  auto status = read_fstring(snapshot, chunk + kChunkContent, value.content);
  if (status != Error::ok) return status;
  status = read_fstring(snapshot, chunk + kChunkComment, value.comment);
  if (status != Error::ok) return status;
  status = read_bool(snapshot, chunk + kChunkNamespaceSet, value.has_name_space);
  if (status != Error::ok) return status;
  if (value.has_name_space) {
    status = read_fstring(snapshot, chunk + kChunkNamespace, value.name_space);
    if (status != Error::ok || !valid_plain_text(value.name_space, false)) {
      return status != Error::ok ? status : Error::invalid_utf8;
    }
  }
  RawShared descriptor{};
  status = read_shared(snapshot, chunk + kChunkClassDescriptor, true, descriptor);
  if (status != Error::ok) return status;
  value.has_class_descriptor = descriptor.object != 0;
  if (class_object != nullptr) *class_object = descriptor.object;
  if (value.has_class_descriptor) {
    status = materialize_class_name(snapshot, descriptor.object, value.class_name);
    if (status != Error::ok) return status;
  }
  if (read_value(snapshot, chunk + kChunkFileLine, RegionKind::immutable_data,
                 value.file_line_number) != Error::ok ||
      read_value(snapshot, chunk + kChunkStart, RegionKind::immutable_data,
                 value.chunk_start) != Error::ok ||
      read_value(snapshot, chunk + kChunkEnd, RegionKind::immutable_data,
                 value.chunk_end) != Error::ok ||
      value.file_line_number < -1 || value.chunk_start < -1 || value.chunk_end < -1 ||
      ((value.chunk_start < 0) != (value.chunk_end < 0)) ||
      (value.chunk_start >= 0 &&
       (value.chunk_start > value.chunk_end ||
        static_cast<std::size_t>(value.chunk_end) > raw_code_units))) {
    return Error::target_layout_drift;
  }
  output = std::move(value);
  return Error::ok;
}

Error materialize_file(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t file,
    TargetFrontendRawFile& output,
    const std::uintptr_t required_class = 0) {
  if (!snapshot.is_data_address(file, kFileStride)) return Error::unreadable_address;
  RawShared module{};
  auto status = read_shared(snapshot, file + kFileModule, false, module);
  if (status != Error::ok) return status;
  TargetFrontendRawFile value{};
  status = materialize_module_name(snapshot, module.object, value.module_name);
  if (status != Error::ok) return status;
  status = read_fstring(snapshot, file + kFileAbsolute, value.absolute_path);
  if (status != Error::ok || !valid_plain_text(value.absolute_path, false)) {
    return status != Error::ok ? status : Error::invalid_utf8;
  }
  status = read_fstring(snapshot, file + kFileRelative, value.relative_path);
  if (status != Error::ok || !valid_plain_text(value.relative_path, false)) {
    return status != Error::ok ? status : Error::invalid_utf8;
  }
  std::size_t raw_units = 0;
  status = read_fstring(snapshot, file + kFileRawCode, value.raw_code, &raw_units);
  if (status != Error::ok) return status;
  status = read_fstring(snapshot, file + kFileProcessedCode, value.processed_code);
  if (status != Error::ok) return status;

  RawArray generated{};
  status = read_array(snapshot, file + kFileGeneratedCode, sizeof(RawArray),
                      kMaximumContainerItems, generated);
  if (status != Error::ok) return status;
  value.generated_code.reserve(static_cast<std::size_t>(generated.num));
  for (std::int32_t index = 0; index < generated.num; ++index) {
    std::string text;
    status = read_fstring(snapshot, generated.data + static_cast<std::size_t>(index) *
                                                    sizeof(RawArray), text);
    if (status != Error::ok) return status;
    value.generated_code.push_back(std::move(text));
  }

  RawArray blocks{};
  status = read_array(snapshot, file + kFileChunkBlocks, sizeof(std::uintptr_t),
                      (kMaximumChunks + kChunkElementsPerBlock - 1) /
                          kChunkElementsPerBlock,
                      blocks);
  if (status != Error::ok) return status;
  std::int32_t chunk_count = 0;
  status = read_value(snapshot, file + kFileChunkCount, RegionKind::immutable_data,
                      chunk_count);
  if (status != Error::ok || chunk_count < 0 ||
      static_cast<std::size_t>(chunk_count) > kMaximumChunks) {
    return Error::invalid_container;
  }
  const std::size_t required_blocks =
      (static_cast<std::size_t>(chunk_count) + kChunkElementsPerBlock - 1) /
      kChunkElementsPerBlock;
  if (static_cast<std::size_t>(blocks.num) != required_blocks) {
    return Error::invalid_container;
  }
  value.chunks.reserve(static_cast<std::size_t>(chunk_count));
  bool required_class_seen = required_class == 0;
  for (std::int32_t index = 0; index < chunk_count; ++index) {
    const std::size_t logical = static_cast<std::size_t>(index);
    const std::size_t block_index = logical / kChunkElementsPerBlock;
    const std::size_t element_index = logical % kChunkElementsPerBlock;
    std::uintptr_t block = 0;
    status = read_value(snapshot,
                        blocks.data + block_index * sizeof(std::uintptr_t),
                        RegionKind::immutable_data, block);
    if (status != Error::ok || !snapshot.is_data_address(block)) {
      return Error::invalid_container;
    }
    std::size_t element_offset = 0;
    std::uintptr_t chunk_address = 0;
    if (!multiply_size(element_index, kChunkStride, element_offset) ||
        !add_address(block, element_offset, chunk_address) ||
        !snapshot.is_data_address(chunk_address, kChunkStride)) {
      return Error::invalid_container;
    }
    TargetFrontendRawChunk chunk{};
    std::uintptr_t class_object = 0;
    status = read_chunk(snapshot, chunk_address, raw_units, chunk, &class_object);
    if (status != Error::ok) return status;
    required_class_seen = required_class_seen || class_object == required_class;
    value.chunks.push_back(std::move(chunk));
  }
  if (!required_class_seen) return Error::invalid_shared_owner;

  bool imports_resolved = false;
  bool resolving_imports = false;
  bool load_async = false;
  if (read_bool(snapshot, file + kFileImportsResolved, imports_resolved) != Error::ok ||
      read_bool(snapshot, file + kFileResolvingImports, resolving_imports) != Error::ok ||
      read_bool(snapshot, file + kFileLoadAsynchronous, load_async) != Error::ok) {
    return Error::target_layout_drift;
  }
  std::uintptr_t async_handle = 0;
  std::uintptr_t async_size = 0;
  std::uintptr_t async_read = 0;
  if (read_value(snapshot, file + kFileAsyncReadHandle, RegionKind::immutable_data,
                 async_handle) != Error::ok ||
      read_value(snapshot, file + kFileAsyncSizeRequest, RegionKind::immutable_data,
                 async_size) != Error::ok ||
      read_value(snapshot, file + kFileAsyncReadRequest, RegionKind::immutable_data,
                 async_read) != Error::ok ||
      resolving_imports || load_async || async_handle != 0 || async_size != 0 ||
      async_read != 0) {
    return Error::unresolved_semantics;
  }
  (void)imports_resolved;
  output = std::move(value);
  return Error::ok;
}

std::string join_generated(const std::vector<std::string>& values) {
  std::string joined;
  for (const auto& value : values) {
    joined += "\n\n";
    joined += value;
  }
  return joined;
}

Error object_path(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t object,
    std::string& path) {
  std::vector<std::string> names;
  std::set<std::uintptr_t> seen;
  auto cursor = object;
  while (cursor != 0) {
    if (names.size() == kMaximumObjectDepth || !seen.insert(cursor).second ||
        !snapshot.is_data_address(cursor, kUObjectOuter + sizeof(std::uintptr_t))) {
      return Error::cyclic_ownership;
    }
    std::uintptr_t vtable = 0;
    std::int32_t internal_index = 0;
    std::uintptr_t object_class = 0;
    TargetRawFName name{};
    std::uintptr_t outer = 0;
    if (read_value(snapshot, cursor + kUObjectVtable, RegionKind::immutable_data, vtable) !=
            Error::ok ||
        read_value(snapshot, cursor + kUObjectInternalIndex, RegionKind::immutable_data,
                   internal_index) != Error::ok ||
        read_value(snapshot, cursor + kUObjectClass, RegionKind::immutable_data,
                   object_class) != Error::ok ||
        read_value(snapshot, cursor + kUObjectName, RegionKind::immutable_data, name) !=
            Error::ok ||
        read_value(snapshot, cursor + kUObjectOuter, RegionKind::immutable_data, outer) !=
            Error::ok ||
        !snapshot.is_image_address(vtable) || internal_index < 0 || object_class == 0 ||
        !snapshot.is_data_address(object_class)) {
      return Error::target_layout_drift;
    }
    std::string spelling;
    const auto status = materialize_fname_v1(snapshot, name, spelling);
    if (status != Error::ok || !valid_plain_text(spelling, false)) {
      return status != Error::ok ? status : Error::invalid_fname;
    }
    names.push_back(std::move(spelling));
    cursor = outer;
  }
  if (names.size() != 2 || !names.back().starts_with("/Script/") ||
      names.front().find_first_of("./:") != std::string::npos) {
    return Error::unresolved_semantics;
  }
  path = names.back();
  path.push_back('.');
  path += names.front();
  return Error::ok;
}

}  // namespace

TargetFrontendRawError TargetFrontendSnapshot::create(
    const std::uintptr_t primary_image,
    const std::uint32_t primary_image_bytes,
    const std::uint64_t epoch,
    const std::span<const TargetRawRegionInput> regions,
    TargetFrontendSnapshot& output) noexcept {
  try {
    if (primary_image == 0 || primary_image_bytes != kPeSizeOfImage || epoch == 0 ||
        regions.empty() || regions.size() > kMaximumSnapshotRegions ||
        primary_image > std::numeric_limits<std::uintptr_t>::max() - primary_image_bytes) {
      return Error::invalid_argument;
    }
    const auto image_end = primary_image + primary_image_bytes;
    TargetFrontendSnapshot candidate;
    candidate.primary_image_ = primary_image;
    candidate.primary_image_bytes_ = primary_image_bytes;
    candidate.epoch_ = epoch;
    std::size_t total_bytes = 0;
    candidate.regions_.reserve(regions.size());
    for (const auto& input : regions) {
      if (input.target_address == 0 || input.bytes == nullptr || input.byte_count == 0 ||
          input.target_address >
              std::numeric_limits<std::uintptr_t>::max() - input.byte_count ||
          (input.kind != RegionKind::primary_image &&
           input.kind != RegionKind::immutable_data) ||
          input.byte_count > kMaximumSnapshotBytes ||
          total_bytes > kMaximumSnapshotBytes - input.byte_count) {
        return Error::invalid_snapshot;
      }
      const auto end = input.target_address + input.byte_count;
      const bool in_image =
          input.target_address >= primary_image && end <= image_end;
      const bool overlaps_image = input.target_address < image_end && end > primary_image;
      if ((input.kind == RegionKind::primary_image && !in_image) ||
          (input.kind == RegionKind::immutable_data && overlaps_image)) {
        return Error::wrong_region_kind;
      }
      TargetFrontendSnapshot::Region region{};
      region.target_address = input.target_address;
      region.kind = input.kind;
      region.bytes.assign(input.bytes, input.bytes + input.byte_count);
      candidate.regions_.push_back(std::move(region));
      total_bytes += input.byte_count;
    }
    std::sort(candidate.regions_.begin(), candidate.regions_.end(),
              [](const Region& left, const Region& right) {
                return left.target_address < right.target_address;
              });
    for (std::size_t index = 1; index < candidate.regions_.size(); ++index) {
      const auto& previous = candidate.regions_[index - 1];
      if (previous.target_address + previous.bytes.size() >
          candidate.regions_[index].target_address) {
        return Error::invalid_snapshot;
      }
    }
    output = std::move(candidate);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

const TargetFrontendSnapshot::Region* TargetFrontendSnapshot::find_region(
    const std::uintptr_t address,
    const std::size_t bytes) const noexcept {
  if (address == 0 || bytes == 0 || address > std::numeric_limits<std::uintptr_t>::max() - bytes) {
    return nullptr;
  }
  const auto found = std::upper_bound(
      regions_.begin(), regions_.end(), address,
      [](const std::uintptr_t value, const Region& region) {
        return value < region.target_address;
      });
  if (found == regions_.begin()) return nullptr;
  const auto& region = *std::prev(found);
  return address >= region.target_address &&
                 address - region.target_address <= region.bytes.size() &&
                 bytes <= region.bytes.size() - (address - region.target_address)
             ? &region
             : nullptr;
}

TargetFrontendRawError TargetFrontendSnapshot::read(
    const std::uintptr_t address,
    const std::span<std::byte> output,
    const TargetRawRegionKind required_kind) const noexcept {
  if (output.empty()) return Error::invalid_argument;
  const auto* region = find_region(address, output.size());
  if (region == nullptr) return Error::unreadable_address;
  if (region->kind != required_kind) return Error::wrong_region_kind;
  std::memcpy(output.data(),
              region->bytes.data() + (address - region->target_address), output.size());
  return Error::ok;
}

TargetFrontendRawError TargetFrontendSnapshot::read_any(
    const std::uintptr_t address,
    const std::span<std::byte> output) const noexcept {
  if (output.empty()) return Error::invalid_argument;
  const auto* region = find_region(address, output.size());
  if (region == nullptr) return Error::unreadable_address;
  std::memcpy(output.data(),
              region->bytes.data() + (address - region->target_address), output.size());
  return Error::ok;
}

bool TargetFrontendSnapshot::is_image_address(
    const std::uintptr_t address,
    const std::size_t bytes) const noexcept {
  if (address < primary_image_ || bytes == 0 ||
      address > std::numeric_limits<std::uintptr_t>::max() - bytes) {
    return false;
  }
  return address - primary_image_ <= primary_image_bytes_ &&
         bytes <= primary_image_bytes_ - (address - primary_image_);
}

bool TargetFrontendSnapshot::is_data_address(
    const std::uintptr_t address,
    const std::size_t bytes) const noexcept {
  const auto* region = find_region(address, bytes);
  return region != nullptr && region->kind == RegionKind::immutable_data;
}

TargetFrontendRawError materialize_fname_v1(
    const TargetFrontendSnapshot& snapshot,
    const TargetRawFName raw,
    std::string& spelling) noexcept {
  try {
    const std::uint32_t block_index = raw.comparison_index >> kFNameBlockShift;
    if (block_index >= kMaximumFNameBlocks) return Error::invalid_fname;
    const std::uint32_t entry_offset =
        (raw.comparison_index & kFNameOffsetMask) * kFNameEntryStride;
    std::uintptr_t block = 0;
    const auto block_pointer = snapshot.primary_image() + kFNamePoolRva + kFNamePoolBlocks +
                               static_cast<std::size_t>(block_index) *
                                   sizeof(std::uintptr_t);
    auto status = read_value(snapshot, block_pointer, RegionKind::primary_image, block);
    // The pointer table is captured from the pinned image, while the immutable snapshot claims
    // only the exact referenced entries. The beginning of a pool block is therefore not itself a
    // required region unless an entry starts at offset zero. Validate the pointer shape here and
    // let the header/character reads below prove that the computed entry was explicitly claimed.
    if (status != Error::ok || block == 0 || snapshot.is_image_address(block)) {
      return Error::invalid_fname;
    }
    std::uintptr_t entry = 0;
    if (!add_address(block, entry_offset, entry)) return Error::address_overflow;
    std::uint16_t header = 0;
    status = read_value(snapshot, entry, RegionKind::immutable_data, header);
    if (status != Error::ok) return Error::invalid_fname;
    const std::size_t length = header >> kFNameHeaderLengthShift;
    if (length == 0 || length > 1023) return Error::invalid_fname;
    if ((header & kFNameWideMask) != 0) {
      std::vector<std::uint16_t> characters(length);
      status = snapshot.read(
          entry + sizeof(header),
          {reinterpret_cast<std::byte*>(characters.data()),
           characters.size() * sizeof(characters.front())},
          RegionKind::immutable_data);
      if (status != Error::ok ||
          std::find(characters.begin(), characters.end(), std::uint16_t{0}) !=
              characters.end()) {
        return Error::invalid_fname;
      }
      status = utf16_to_utf8(characters, spelling);
      if (status != Error::ok) return status;
    } else {
      std::vector<std::byte> characters(length);
      status = snapshot.read(entry + sizeof(header), characters,
                             RegionKind::immutable_data);
      if (status != Error::ok) return Error::invalid_fname;
      spelling.clear();
      spelling.reserve(length);
      for (const auto byte : characters) {
        const auto value = std::to_integer<std::uint8_t>(byte);
        if (value < 0x20 || value > 0x7e) return Error::invalid_fname;
        spelling.push_back(static_cast<char>(value));
      }
    }
    if (raw.number != 0) {
      spelling.push_back('_');
      spelling += std::to_string(raw.number - 1);
    }
    return valid_plain_text(spelling, false) ? Error::ok : Error::invalid_fname;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_graph_hook_bindings_v1(
    const TargetFrontendSnapshot& snapshot,
    TargetFrontendGraphHookBindings& bindings) noexcept {
  try {
    TargetFrontendGraphHookBindings projected{};
    const auto class_object =
        snapshot.primary_image() + frontend_target_layout::class_analyze_delegate_rva;
    std::uintptr_t class_list = 0;
    std::int32_t class_num = 0;
    std::int32_t class_capacity = 0;
    std::int32_t class_broadcast_count = 0;
    if (read_value(snapshot, class_object + kGraphDelegateInvocationList,
                   RegionKind::primary_image, class_list) != Error::ok ||
        read_value(snapshot, class_object + kGraphDelegateNum,
                   RegionKind::primary_image, class_num) != Error::ok ||
        read_value(snapshot, class_object + kGraphDelegateMax,
                   RegionKind::primary_image, class_capacity) != Error::ok ||
        read_value(snapshot, class_object + kGraphDelegateBroadcastCount,
                   RegionKind::primary_image, class_broadcast_count) != Error::ok) {
      return Error::unreadable_address;
    }
    projected.class_analyze_state = {
        class_list, class_num, class_capacity, 0, class_broadcast_count};
    projected.diagnostic_delegate = 1;
    bindings = projected;
    if (class_num < 0 || class_capacity < class_num || class_capacity > 1'000'000 ||
        class_broadcast_count != 0 ||
        (class_capacity == 0 ? class_list != 0 : class_list == 0)) {
      return Error::target_layout_drift;
    }
    for (std::int32_t index = 0; index < class_num; ++index) {
      std::uintptr_t object = 0;
      std::uintptr_t function = 0;
      const auto entry = class_list + static_cast<std::size_t>(index) * 0x10;
      if (read_value(snapshot, entry, RegionKind::immutable_data, object) != Error::ok ||
          read_value(snapshot, entry + sizeof(object), RegionKind::immutable_data,
                     function) != Error::ok) {
        return Error::unreadable_address;
      }
      // The pinned Broadcast CFG invokes only pairs for which both words are nonzero. A
      // half-cleared slot is a valid removed entry and never becomes semantic output.  The
      // second word is an opaque UE delegate payload, not a replayable code capability; actual
      // ClassAnalyze invocations are captured independently at the pinned broadcast callsite.
      if (object != 0 && function != 0) {
        ++projected.class_analyze_active_bindings;
      }
    }
    projected.class_analyze_bound = projected.class_analyze_active_bindings != 0;
    std::uint32_t delegate_index = 0;
    for (const auto rva : {frontend_target_layout::process_chunks_delegate_rva,
                           frontend_target_layout::post_process_code_delegate_rva}) {
      ++delegate_index;
      const auto object = snapshot.primary_image() + rva;
      std::uintptr_t invocation_list = 0;
      std::int32_t num = 0;
      std::int32_t capacity = 0;
      std::int32_t compaction_threshold = 0;
      std::int32_t broadcast_count = 0;
      if (read_value(snapshot, object + kGraphDelegateInvocationList,
                     RegionKind::primary_image, invocation_list) != Error::ok ||
          read_value(snapshot, object + kGraphDelegateNum, RegionKind::primary_image, num) !=
              Error::ok ||
          read_value(snapshot, object + kGraphDelegateMax, RegionKind::primary_image,
                     capacity) != Error::ok ||
          read_value(snapshot, object + kGraphDelegateCompactionThreshold,
                     RegionKind::primary_image, compaction_threshold) != Error::ok ||
          read_value(snapshot, object + kGraphDelegateBroadcastCount,
                     RegionKind::primary_image, broadcast_count) != Error::ok) {
        return Error::unreadable_address;
      }
      auto& raw_state = delegate_index == 1 ? projected.process_chunks_state
                                            : projected.post_process_code_state;
      raw_state = {invocation_list, num, capacity, compaction_threshold,
                   broadcast_count};
      projected.diagnostic_delegate = delegate_index + 1;
      bindings = projected;
      if (invocation_list != 0 || num != 0 || capacity != 0) {
        return Error::unresolved_semantics;
      }
      if (compaction_threshold != 2 || broadcast_count != 0) {
        return Error::target_layout_drift;
      }
    }
    bindings = projected;
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_graph_hook_config_v1(
    const TargetFrontendSnapshot& snapshot,
    FrontendPreprocessorConfig& config) noexcept {
  try {
    if (config.process_chunks_bound || !config.process_chunks_captures.empty() ||
        config.post_process_code_bound || !config.post_process_code_captures.empty()) {
      return Error::invalid_argument;
    }
    TargetFrontendGraphHookBindings bindings{};
    const auto status = materialize_graph_hook_bindings_v1(snapshot, bindings);
    if (status != Error::ok) return status;
    if (bindings.process_chunks_bound || bindings.post_process_code_bound) {
      return Error::unresolved_semantics;
    }
    config.class_analyze_bound = bindings.class_analyze_bound;
    config.process_chunks_bound = false;
    config.process_chunks_captures.clear();
    config.post_process_code_bound = false;
    config.post_process_code_captures.clear();
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_preprocessor_flags_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t preprocessor,
    std::vector<FrontendFlag>& flags) noexcept {
  try {
    std::vector<std::uintptr_t> slots;
    auto status = read_sparse_slots(snapshot, preprocessor, kMapFlagElementStride,
                                    kMaximumContainerItems, slots);
    if (status != Error::ok) return status;
    std::vector<FrontendFlag> output;
    output.reserve(slots.size());
    for (const auto slot : slots) {
      FrontendFlag flag{};
      status = read_fstring(snapshot, slot, flag.name);
      if (status != Error::ok || !valid_plain_text(flag.name, false)) {
        return status != Error::ok ? status : Error::invalid_utf8;
      }
      status = read_bool(snapshot, slot + kMapFlagValue, flag.value);
      if (status != Error::ok) return status;
      output.push_back(std::move(flag));
    }
    std::sort(output.begin(), output.end(),
              [](const FrontendFlag& left, const FrontendFlag& right) {
                return left.name < right.name;
              });
    for (std::size_t index = 1; index < output.size(); ++index) {
      if (output[index - 1].name == output[index].name) return Error::duplicate_identity;
    }
    flags = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_settings_flags_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t manager,
    std::vector<FrontendFlag>& flags) noexcept {
  try {
    std::uintptr_t settings = 0;
    auto status = read_value(
        snapshot, manager + frontend_target_layout::manager_settings,
        RegionKind::immutable_data, settings);
    if (status != Error::ok || settings == 0 || !snapshot.is_data_address(settings)) {
      return status != Error::ok ? status : Error::target_layout_drift;
    }
    RawArray array{};
    status = read_array(
        snapshot, settings + frontend_target_layout::settings_preprocessor_flags,
        sizeof(RawArray), kMaximumContainerItems, array);
    if (status != Error::ok) return status;
    std::vector<FrontendFlag> output;
    output.reserve(static_cast<std::size_t>(array.num));
    for (std::int32_t index = 0; index < array.num; ++index) {
      FrontendFlag flag{};
      status = read_fstring(
          snapshot,
          array.data + static_cast<std::size_t>(index) * sizeof(RawArray),
          flag.name);
      if (status != Error::ok || !valid_plain_text(flag.name, false)) {
        return status != Error::ok ? status : Error::invalid_utf8;
      }
      flag.value = true;
      output.push_back(std::move(flag));
    }
    std::sort(output.begin(), output.end(),
              [](const FrontendFlag& left, const FrontendFlag& right) {
                return left.name < right.name;
              });
    if (std::adjacent_find(
            output.begin(), output.end(),
            [](const FrontendFlag& left, const FrontendFlag& right) {
              return left.name == right.name;
            }) != output.end()) {
      return Error::duplicate_identity;
    }
    flags = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_blueprint_specializations_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t manager,
    std::vector<std::string>& specializations) noexcept {
  try {
    std::vector<std::uintptr_t> slots;
    auto status = read_sparse_slots(
        snapshot,
        manager + frontend_target_layout::manager_blueprint_specializations,
        kSetStringElementStride,
        kMaximumContainerItems,
        slots);
    if (status != Error::ok) return status;
    std::vector<std::string> output;
    output.reserve(slots.size());
    for (const auto slot : slots) {
      std::string value;
      status = read_fstring(snapshot, slot, value);
      if (status != Error::ok || !valid_plain_text(value, false)) {
        return status != Error::ok ? status : Error::invalid_utf8;
      }
      output.push_back(std::move(value));
    }
    std::sort(output.begin(), output.end());
    if (std::adjacent_find(output.begin(), output.end()) != output.end()) {
      return Error::duplicate_identity;
    }
    specializations = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_static_fnames_v1(
    const TargetFrontendSnapshot& snapshot,
    std::vector<FrontendFNameComparison>& names) noexcept {
  try {
    RawArray array{};
    auto status = read_array(
        snapshot,
        snapshot.primary_image() + frontend_target_layout::static_names_rva,
        sizeof(TargetRawFName),
        kMaximumContainerItems,
        array);
    if (status != Error::ok) return status;
    std::vector<FrontendFNameComparison> output;
    for (std::int32_t index = 0; index < array.num; ++index) {
      TargetRawFName raw{};
      status = read_value(snapshot,
                          array.data + static_cast<std::size_t>(index) * sizeof(raw),
                          RegionKind::immutable_data, raw);
      if (status != Error::ok) return status;
      std::string spelling;
      status = materialize_fname_v1(snapshot, raw, spelling);
      if (status != Error::ok) return status;
      if (std::none_of(spelling.begin(), spelling.end(), [](const char value) {
            return static_cast<unsigned char>(value) >= 0x80;
          })) {
        continue;
      }
      FrontendFNameComparison projection{};
      const auto projected = make_fname_comparison_key_v1(
          spelling, raw.comparison_index, projection);
      if (projected != FrontendObserverError::ok) return Error::invalid_fname;
      output.push_back(std::move(projection));
    }
    std::sort(output.begin(), output.end(),
              [](const FrontendFNameComparison& left,
                 const FrontendFNameComparison& right) {
                return left.spelling < right.spelling;
              });
    for (std::size_t index = 1; index < output.size(); ++index) {
      if (output[index - 1].spelling == output[index].spelling) {
        return Error::duplicate_identity;
      }
    }
    names = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_uclass_witness_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t uclass,
    const std::string_view angelscript_type_name,
    const std::uint64_t property_offset,
    FrontendNativeClassWitness& witness) noexcept {
  try {
    if (uclass == 0 || !valid_plain_text(angelscript_type_name, false) ||
        property_offset > static_cast<std::uint64_t>(std::numeric_limits<std::int32_t>::max())) {
      return Error::invalid_argument;
    }
    FrontendNativeClassWitness output{};
    output.angelscript_type_name.assign(angelscript_type_name);
    output.property_offset = property_offset;
    std::set<std::uintptr_t> seen;
    auto cursor = uclass;
    std::int32_t derived_properties_size = std::numeric_limits<std::int32_t>::max();
    while (cursor != 0) {
      if (output.ancestry_paths.size() == kMaximumObjectDepth ||
          !seen.insert(cursor).second ||
          !snapshot.is_data_address(cursor, kUStructPropertiesSize + sizeof(std::int32_t))) {
        return Error::cyclic_ownership;
      }
      std::string path;
      auto status = object_path(snapshot, cursor, path);
      if (status != Error::ok) return status;
      std::int32_t properties_size = 0;
      std::uintptr_t super = 0;
      if (read_value(snapshot, cursor + kUStructPropertiesSize,
                     RegionKind::immutable_data, properties_size) != Error::ok ||
          read_value(snapshot, cursor + kUStructSuper, RegionKind::immutable_data, super) !=
              Error::ok ||
          properties_size < 0 ||
          properties_size > derived_properties_size ||
          (cursor == uclass && property_offset > static_cast<std::uint64_t>(properties_size)) ||
          (super != 0 && !snapshot.is_data_address(super))) {
        return Error::target_layout_drift;
      }
      output.ancestry_paths.push_back(std::move(path));
      derived_properties_size = properties_size;
      cursor = super;
    }
    if (output.ancestry_paths.empty()) return Error::unresolved_semantics;
    output.unreal_class_path = output.ancestry_paths.front();
    witness = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_native_class_witness_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t uclass,
    const std::string_view angelscript_type_name,
    FrontendNativeClassWitness& witness) noexcept {
  std::int32_t property_offset = -1;
  if (uclass == 0 ||
      read_value(snapshot, uclass + kUStructPropertiesSize,
                 RegionKind::immutable_data, property_offset) != Error::ok ||
      property_offset < 0) {
    return Error::target_layout_drift;
  }
  return materialize_uclass_witness_v1(
      snapshot, uclass, angelscript_type_name,
      static_cast<std::uint64_t>(property_offset), witness);
}

TargetFrontendRawError materialize_uobject_class_path_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t object,
    std::string& class_path) noexcept {
  try {
    std::uintptr_t object_class = 0;
    if (object == 0 ||
        read_value(snapshot, object + kUObjectClass, RegionKind::immutable_data,
                   object_class) != Error::ok ||
        object_class == 0) {
      return Error::target_layout_drift;
    }
    std::string path;
    const auto status = object_path(snapshot, object_class, path);
    if (status != Error::ok) return status;
    class_path = std::move(path);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_class_native_super_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t class_descriptor_shared,
    TargetFrontendNativeSuperRaw& native_super) noexcept {
  try {
    RawShared descriptor{};
    auto status = read_shared(snapshot, class_descriptor_shared, false, descriptor);
    if (status != Error::ok ||
        !snapshot.is_data_address(descriptor.object, kClassNamespaceSet + 1)) {
      return status != Error::ok ? status : Error::unreadable_address;
    }
    bool is_code = false;
    status = read_bool(snapshot, descriptor.object + kClassSuperIsCode, is_code);
    if (status != Error::ok) return status;
    if (!is_code) {
      native_super = {};
      return Error::ok;
    }
    std::string name;
    status = read_fstring(snapshot, descriptor.object + kClassSuperName, name);
    if (status != Error::ok || !valid_plain_text(name, false)) {
      return status != Error::ok ? status : Error::invalid_utf8;
    }
    std::uintptr_t uclass = 0;
    std::int32_t property_offset = -1;
    if (read_value(snapshot, descriptor.object + kClassCodeSuper,
                   RegionKind::immutable_data, uclass) != Error::ok ||
        uclass == 0 ||
        read_value(snapshot, uclass + kUStructPropertiesSize,
                   RegionKind::immutable_data, property_offset) != Error::ok ||
        property_offset < 0) {
      return Error::target_layout_drift;
    }
    TargetFrontendNativeSuperRaw value{};
    value.present = true;
    status = materialize_uclass_witness_v1(
        snapshot, uclass, name, static_cast<std::uint64_t>(property_offset), value.witness);
    if (status != Error::ok) return status;
    native_super = std::move(value);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_preprocessor_files_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t preprocessor,
    std::vector<TargetFrontendRawFile>& files) noexcept {
  try {
    RawArray array{};
    auto status = read_array(snapshot,
                             preprocessor + frontend_target_layout::preprocessor_files,
                             kFileStride, kMaximumFiles, array);
    if (status != Error::ok) return status;
    std::vector<TargetFrontendRawFile> output;
    std::vector<std::pair<std::string, std::uintptr_t>> module_identities;
    std::set<std::string> relative_paths;
    output.reserve(static_cast<std::size_t>(array.num));
    for (std::int32_t index = 0; index < array.num; ++index) {
      const auto file_address =
          array.data + static_cast<std::size_t>(index) * kFileStride;
      RawShared module{};
      status = read_shared(snapshot, file_address + kFileModule, false, module);
      if (status != Error::ok) return status;
      TargetFrontendRawFile file{};
      status = materialize_file(snapshot, file_address, file);
      if (status != Error::ok) return status;
      const auto same_name = std::find_if(
          module_identities.begin(), module_identities.end(), [&](const auto& identity) {
            return identity.first == file.module_name;
          });
      if ((same_name != module_identities.end() && same_name->second != module.object) ||
          std::any_of(module_identities.begin(), module_identities.end(),
                      [&](const auto& identity) {
                        return identity.second == module.object &&
                               identity.first != file.module_name;
                      }) ||
          !relative_paths.insert(file.relative_path).second) {
        return Error::duplicate_identity;
      }
      if (same_name == module_identities.end()) {
        module_identities.emplace_back(file.module_name, module.object);
      }
      output.push_back(std::move(file));
    }
    files = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_preprocessor_graph_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t preprocessor,
    const TargetFrontendGraphSource source,
    std::vector<FrontendGraphModule>& modules) noexcept {
  try {
    if (source != TargetFrontendGraphSource::chunk_content &&
        source != TargetFrontendGraphSource::processed_code) {
      return Error::invalid_argument;
    }
    std::vector<TargetFrontendRawFile> files;
    auto status = materialize_preprocessor_files_v1(snapshot, preprocessor, files);
    if (status != Error::ok) return status;
    std::vector<FrontendGraphModule> output;
    for (const auto& file : files) {
      auto found = std::find_if(output.begin(), output.end(), [&](const auto& module) {
        return module.module_name == file.module_name;
      });
      if (found == output.end()) {
        output.push_back({file.module_name, {}, {}});
        found = std::prev(output.end());
      }
      if (std::any_of(found->sections.begin(), found->sections.end(), [&](const auto& section) {
            return section.relative_path == file.relative_path;
          })) {
        return Error::duplicate_identity;
      }
      std::string code;
      if (source == TargetFrontendGraphSource::chunk_content) {
        for (const auto& chunk : file.chunks) code += chunk.content;
      } else {
        code = file.processed_code;
      }
      found->sections.push_back({file.relative_path, std::move(code)});
      found->generated_declarations += join_generated(file.generated_code);
    }
    modules = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_module_descriptor_graph_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t descriptor_array,
    std::vector<FrontendGraphModule>& modules) noexcept {
  try {
    RawArray descriptors{};
    auto status = read_array(snapshot, descriptor_array, sizeof(RawShared),
                             kMaximumFiles, descriptors);
    if (status != Error::ok) return status;
    std::vector<FrontendGraphModule> output;
    std::set<std::string> names;
    output.reserve(static_cast<std::size_t>(descriptors.num));
    for (std::int32_t index = 0; index < descriptors.num; ++index) {
      RawShared module{};
      status = read_shared(snapshot,
                           descriptors.data + static_cast<std::size_t>(index) *
                                                  sizeof(RawShared),
                           false, module);
      if (status != Error::ok) return status;
      FrontendGraphModule projection{};
      status = materialize_module_name(snapshot, module.object, projection.module_name);
      if (status != Error::ok || !names.insert(projection.module_name).second) {
        return status != Error::ok ? status : Error::duplicate_identity;
      }
      RawArray sections{};
      status = read_array(snapshot, module.object + kModuleCode, kCodeSectionStride,
                          kMaximumContainerItems, sections);
      if (status != Error::ok) return status;
      std::uint64_t combined = 0;
      std::set<std::string> paths;
      projection.sections.reserve(static_cast<std::size_t>(sections.num));
      for (std::int32_t section_index = 0; section_index < sections.num; ++section_index) {
        const auto section = sections.data +
                             static_cast<std::size_t>(section_index) * kCodeSectionStride;
        FrontendGraphSection value{};
        std::string absolute;
        std::vector<std::uint16_t> raw_code;
        status = read_fstring(snapshot, section + kCodeSectionRelative, value.relative_path);
        if (status != Error::ok || !valid_plain_text(value.relative_path, false) ||
            !paths.insert(value.relative_path).second) {
          return status != Error::ok ? status : Error::duplicate_identity;
        }
        status = read_fstring(snapshot, section + kCodeSectionAbsolute, absolute);
        if (status != Error::ok || !valid_plain_text(absolute, false)) {
          return status != Error::ok ? status : Error::invalid_utf8;
        }
        status = read_raw_fstring_units(snapshot, section + kCodeSectionCode, raw_code);
        if (status != Error::ok) return status;
        status = utf16_to_utf8(raw_code, value.code);
        if (status != Error::ok) return status;
        std::int64_t captured_hash = 0;
        if (read_value(snapshot, section + kCodeSectionHash, RegionKind::immutable_data,
                       captured_hash) != Error::ok) {
          return Error::unreadable_address;
        }
        const auto computed_hash = raw_code.empty()
                                       ? 0
                                       : xxh64(
                                             {reinterpret_cast<const std::byte*>(raw_code.data()),
                                              raw_code.size() * sizeof(raw_code.front())});
        if (static_cast<std::uint64_t>(captured_hash) != computed_hash) {
          return Error::target_layout_drift;
        }
        combined ^= computed_hash;
        projection.sections.push_back(std::move(value));
      }
      std::int64_t module_hash = 0;
      if (read_value(snapshot, module.object + kModuleCodeHash,
                     RegionKind::immutable_data, module_hash) != Error::ok ||
          static_cast<std::uint64_t>(module_hash) != combined) {
        return Error::target_layout_drift;
      }
      output.push_back(std::move(projection));
    }
    modules = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

TargetFrontendRawError materialize_class_analyze_frame_v1(
    const TargetFrontendSnapshot& snapshot,
    const std::uintptr_t file,
    const std::uintptr_t generated_statics_fstring,
    const std::uintptr_t class_descriptor_shared,
    const std::uintptr_t has_statics,
    FrontendClassFrame& frame) noexcept {
  try {
    RawShared descriptor{};
    auto status = read_shared(snapshot, class_descriptor_shared, false, descriptor);
    if (status != Error::ok) return status;
    TargetFrontendRawFile raw_file{};
    status = materialize_file(snapshot, file, raw_file, descriptor.object);
    if (status != Error::ok) return status;
    FrontendClassFrame output{};
    output.module_name = raw_file.module_name;
    output.source = raw_file.raw_code;
    status = materialize_class_name(snapshot, descriptor.object, output.class_name);
    if (status != Error::ok) return status;
    status = read_fstring(snapshot, descriptor.object + kClassComposeOnto,
                          output.compose_onto_class);
    if (status != Error::ok) return status;
    bool has_namespace = false;
    status = read_bool(snapshot, descriptor.object + kClassNamespaceSet, has_namespace);
    if (status != Error::ok) return status;
    if (has_namespace) {
      status = read_fstring(snapshot, descriptor.object + kClassNamespace,
                            output.name_space);
      if (status != Error::ok || !valid_plain_text(output.name_space, false)) {
        return status != Error::ok ? status : Error::invalid_utf8;
      }
    }
    status = read_fstring(snapshot, generated_statics_fstring, output.generated_statics);
    if (status != Error::ok) return status;
    status = read_bool(snapshot, has_statics, output.has_statics);
    if (status != Error::ok) return status;
    frame = std::move(output);
    return Error::ok;
  } catch (...) {
    return Error::limit_exceeded;
  }
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

class RawFixture final {
 public:
  static constexpr std::uintptr_t kImage = 0x0000000140000000ull;
  static constexpr std::uintptr_t kData = 0x0000000200000000ull;

  RawFixture()
      : data_(1024u * 1024u),
        pool_(0x20),
        static_names_(sizeof(RawArray)),
        class_delegate_(kGraphDelegateBytes),
        graph_delegates_(2 * kGraphDelegateBytes) {
    name_block_ = kData;
    cursor_ = 0x20000;
    store_bytes(pool_, kFNamePoolBlocks, name_block_);
    store_bytes(graph_delegates_, kGraphDelegateCompactionThreshold, std::int32_t{2});
    store_bytes(graph_delegates_,
                kGraphDelegateBytes + kGraphDelegateCompactionThreshold,
                std::int32_t{2});
  }

  bool build() {
    const auto meta_name = add_fname("Class");
    const auto mod_package_name = add_fname("/Script/Mod");
    const auto engine_package_name = add_fname("/Script/Engine");
    const auto core_package_name = add_fname("/Script/CoreUObject");
    const auto mod_actor_name = add_fname("ModActor");
    const auto actor_name = add_fname("Actor");
    const auto object_name = add_fname("Object");
    const auto ascii_name = add_fname("PlainName");
    const auto unicode_name = add_fname(u"Gr\u00f6\u00dfe");
    if (unicode_name.comparison_index == 0 && name_cursor_ == 0) return false;

    meta_class_ = allocate(0x60, 8);
    initialize_object(meta_class_, meta_name, 0, meta_class_);
    mod_package_ = allocate(0x60, 8);
    engine_package_ = allocate(0x60, 8);
    core_package_ = allocate(0x60, 8);
    initialize_object(mod_package_, mod_package_name, 0, meta_class_);
    initialize_object(engine_package_, engine_package_name, 0, meta_class_);
    initialize_object(core_package_, core_package_name, 0, meta_class_);
    object_class_ = allocate(0x60, 8);
    actor_class_ = allocate(0x60, 8);
    mod_actor_class_ = allocate(0x60, 8);
    initialize_object(object_class_, object_name, core_package_, meta_class_, 0, 32);
    initialize_object(actor_class_, actor_name, engine_package_, meta_class_, object_class_, 96);
    initialize_object(mod_actor_class_, mod_actor_name, mod_package_, meta_class_, actor_class_,
                      256);

    class_descriptor_ = allocate(0x120, 8);
    put_fstring(class_descriptor_ + kClassName, u"AModActor");
    put_fstring(class_descriptor_ + kClassSuperName, u"AActor");
    store(class_descriptor_ + kClassCodeSuper, actor_class_);
    store(class_descriptor_ + kClassSuperIsCode, std::uint8_t{1});
    put_fstring(class_descriptor_ + kClassComposeOnto, u"AParent");
    put_fstring(class_descriptor_ + kClassNamespace, u"Example");
    store(class_descriptor_ + kClassNamespaceSet, std::uint8_t{1});
    class_shared_ = make_shared(class_descriptor_);

    module_ = allocate(0x40, 8);
    put_fstring(module_ + kModuleName, u"Mods.Sample");
    module_shared_ = make_shared(module_);
    const auto sections = allocate(kCodeSectionStride, 8);
    put_fstring(sections + kCodeSectionRelative, u"Mods/Sample.as");
    put_fstring(sections + kCodeSectionAbsolute, u"C:/fixture/Mods/Sample.as");
    const std::u16string code = u"class AModActor : AActor {}";
    put_fstring(sections + kCodeSectionCode, code);
    const auto code_hash = fixture_xxh(code);
    store(sections + kCodeSectionHash, static_cast<std::int64_t>(code_hash));
    put_array(module_ + kModuleCode, sections, 1, 1);
    store(module_ + kModuleCodeHash, static_cast<std::int64_t>(code_hash));

    file_ = allocate(kFileStride, 8);
    store(file_ + kFileModule, module_shared_);
    put_fstring(file_ + kFileAbsolute, u"C:/fixture/Mods/Sample.as");
    put_fstring(file_ + kFileRelative, u"Mods/Sample.as");
    put_fstring(file_ + kFileRawCode, code);
    put_fstring(file_ + kFileProcessedCode, code);
    const auto generated = allocate(sizeof(RawArray), 8);
    put_fstring(generated, u"void Generated();");
    put_array(file_ + kFileGeneratedCode, generated, 1, 1);
    const auto chunk_block = allocate(kChunkStride, 8);
    store(chunk_block + kChunkType, std::uint8_t{1});
    put_fstring(chunk_block + kChunkContent, code);
    put_fstring(chunk_block + kChunkComment, u"// fixture");
    store(chunk_block + kChunkClassDescriptor, class_shared_);
    put_fstring(chunk_block + kChunkNamespace, u"Example");
    store(chunk_block + kChunkNamespaceSet, std::uint8_t{1});
    store(chunk_block + kChunkFileLine, std::int32_t{1});
    store(chunk_block + kChunkStart, std::int32_t{0});
    store(chunk_block + kChunkEnd, static_cast<std::int32_t>(code.size()));
    const auto chunk_blocks = allocate(sizeof(std::uintptr_t), 8);
    store(chunk_blocks, chunk_block);
    put_array(file_ + kFileChunkBlocks, chunk_blocks, 1, 1);
    store(file_ + kFileChunkCount, std::int32_t{1});
    store(file_ + kFileImportsResolved, std::uint8_t{1});

    preprocessor_ = allocate(0x108, 8);
    const auto flag_elements = allocate(2 * kMapFlagElementStride, 8);
    put_fstring(flag_elements, u"TEST");
    store(flag_elements + kMapFlagValue, std::uint8_t{0});
    put_fstring(flag_elements + kMapFlagElementStride, u"RELEASE");
    store(flag_elements + kMapFlagElementStride + kMapFlagValue, std::uint8_t{1});
    const auto flag_bits = allocate(sizeof(std::uint32_t), alignof(std::uint32_t));
    store(flag_bits, std::uint32_t{3});
    put_sparse(preprocessor_, flag_elements, 2, 2, flag_bits, 2, 32);
    flag_bits_ = preprocessor_ + kSparseInlineAllocationFlags;
    put_array(preprocessor_ + frontend_target_layout::preprocessor_files, file_, 1, 1);

    manager_ = allocate(0x500, 8);
    const auto set_elements = allocate(2 * kSetStringElementStride, 8);
    put_fstring(set_elements, u"int32");
    put_fstring(set_elements + kSetStringElementStride, u"FName");
    const auto set_bits = allocate(sizeof(std::uint32_t), alignof(std::uint32_t));
    store(set_bits, std::uint32_t{3});
    put_sparse(manager_ + frontend_target_layout::manager_blueprint_specializations,
               set_elements, 2, 2, set_bits, 2, 32);

    const auto descriptor_entries = allocate(sizeof(RawShared), 8);
    store(descriptor_entries, module_shared_);
    descriptor_array_ = allocate(sizeof(RawArray), 8);
    put_array(descriptor_array_, descriptor_entries, 1, 1);

    generated_statics_ = allocate(sizeof(RawArray), 8);
    put_fstring(generated_statics_, u"void StaticHelper();");
    class_shared_address_ = allocate(sizeof(RawShared), 8);
    store(class_shared_address_, class_shared_);
    has_statics_ = allocate(1, 1);
    store(has_statics_, std::uint8_t{1});

    const auto static_entries = allocate(2 * sizeof(TargetRawFName), 8);
    store(static_entries, ascii_name);
    store(static_entries + sizeof(TargetRawFName), unicode_name);
    RawArray static_header{static_entries, 2, 2};
    std::memcpy(static_names_.data(), &static_header, sizeof(static_header));
    class_delegate_entry_ = allocate(0x10, 8);
    store(class_delegate_entry_, manager_);
    store(class_delegate_entry_ + 8, kImage + 0x00112230);
    return valid_;
  }

  Error snapshot(TargetFrontendSnapshot& output) const {
    const std::array regions{
        TargetRawRegionInput{kImage + kFNamePoolRva, pool_.data(), pool_.size(),
                             RegionKind::primary_image},
        TargetRawRegionInput{kImage + frontend_target_layout::class_analyze_delegate_rva,
                             class_delegate_.data(), class_delegate_.size(),
                             RegionKind::primary_image},
        TargetRawRegionInput{kImage + frontend_target_layout::process_chunks_delegate_rva,
                             graph_delegates_.data(), graph_delegates_.size(),
                             RegionKind::primary_image},
        TargetRawRegionInput{kImage + frontend_target_layout::static_names_rva,
                             static_names_.data(), static_names_.size(),
                             RegionKind::primary_image},
        TargetRawRegionInput{kData, data_.data(), data_.size(),
                             RegionKind::immutable_data},
    };
    return TargetFrontendSnapshot::create(kImage, kPeSizeOfImage, 7, regions, output);
  }

  template <typename Type>
  void store(const std::uintptr_t address, const Type& value) {
    static_assert(std::is_trivially_copyable_v<Type>);
    if (address < kData || address - kData > data_.size() ||
        sizeof(value) > data_.size() - (address - kData)) {
      valid_ = false;
      return;
    }
    std::memcpy(data_.data() + (address - kData), &value, sizeof(value));
  }

  template <typename Type>
  Type load(const std::uintptr_t address) const {
    Type value{};
    if (address >= kData && address - kData <= data_.size() &&
        sizeof(value) <= data_.size() - (address - kData)) {
      std::memcpy(&value, data_.data() + (address - kData), sizeof(value));
    }
    return value;
  }

  std::uintptr_t preprocessor() const noexcept { return preprocessor_; }
  std::uintptr_t manager() const noexcept { return manager_; }
  std::uintptr_t descriptor_array() const noexcept { return descriptor_array_; }
  std::uintptr_t file() const noexcept { return file_; }
  std::uintptr_t generated_statics() const noexcept { return generated_statics_; }
  std::uintptr_t class_shared_address() const noexcept { return class_shared_address_; }
  std::uintptr_t class_controller() const noexcept { return class_shared_.controller; }
  std::uintptr_t has_statics() const noexcept { return has_statics_; }
  std::uintptr_t mod_actor_class() const noexcept { return mod_actor_class_; }
  std::uintptr_t flag_bits() const noexcept { return flag_bits_; }
  std::uintptr_t actor_class() const noexcept { return actor_class_; }

  void set_process_delegate_pointer(const std::uintptr_t pointer) {
    store_bytes(graph_delegates_, kGraphDelegateInvocationList, pointer);
  }

  void set_class_delegate_bound(const bool bound) {
    store_bytes(class_delegate_, kGraphDelegateInvocationList,
                bound ? class_delegate_entry_ : std::uintptr_t{0});
    store_bytes(class_delegate_, kGraphDelegateNum, std::int32_t{bound ? 1 : 0});
    store_bytes(class_delegate_, kGraphDelegateMax, std::int32_t{bound ? 1 : 0});
  }

 private:
  std::uintptr_t allocate(const std::size_t bytes, const std::size_t alignment) {
    const auto aligned = (cursor_ + alignment - 1) & ~(alignment - 1);
    if (alignment == 0 || (alignment & (alignment - 1)) != 0 || aligned > data_.size() ||
        bytes > data_.size() - aligned) {
      valid_ = false;
      return 0;
    }
    cursor_ = aligned + bytes;
    return kData + aligned;
  }

  template <typename Type>
  static void store_bytes(std::vector<std::byte>& bytes, const std::size_t offset,
                          const Type& value) {
    if (offset <= bytes.size() && sizeof(value) <= bytes.size() - offset) {
      std::memcpy(bytes.data() + offset, &value, sizeof(value));
    }
  }

  TargetRawFName add_fname(const std::string_view value) {
    std::u16string wide;
    wide.reserve(value.size());
    for (const auto character : value) {
      if (static_cast<unsigned char>(character) > 0x7f) {
        valid_ = false;
        return {};
      }
      wide.push_back(static_cast<char16_t>(character));
    }
    return add_fname_impl(wide, false);
  }

  TargetRawFName add_fname(const std::u16string_view value) {
    return add_fname_impl(value, true);
  }

  TargetRawFName add_fname_impl(const std::u16string_view value, const bool wide) {
    if (value.empty() || value.size() > 1023 || (name_cursor_ & 1) != 0) {
      valid_ = false;
      return {};
    }
    const auto comparison = static_cast<std::uint32_t>(name_cursor_ / 2);
    const std::uint16_t header =
        static_cast<std::uint16_t>((value.size() << kFNameHeaderLengthShift) |
                                   (wide ? kFNameWideMask : 0));
    store(name_block_ + name_cursor_, header);
    name_cursor_ += sizeof(header);
    if (wide) {
      for (const auto character : value) {
        store(name_block_ + name_cursor_, static_cast<std::uint16_t>(character));
        name_cursor_ += sizeof(std::uint16_t);
      }
    } else {
      for (const auto character : value) {
        store(name_block_ + name_cursor_, static_cast<std::uint8_t>(character));
        ++name_cursor_;
      }
      if ((name_cursor_ & 1) != 0) ++name_cursor_;
    }
    return {comparison, 0};
  }

  void initialize_object(const std::uintptr_t object, const TargetRawFName name,
                         const std::uintptr_t outer, const std::uintptr_t object_class,
                         const std::uintptr_t super = 0, const std::int32_t properties = 0) {
    store(object + kUObjectVtable, kImage + 0x00123450);
    store(object + kUObjectFlags, std::uint32_t{0});
    store(object + kUObjectInternalIndex, next_object_index_++);
    store(object + kUObjectClass, object_class);
    store(object + kUObjectName, name);
    store(object + kUObjectOuter, outer);
    if (object >= kData && object - kData <= data_.size() &&
        kUStructPropertiesSize + sizeof(properties) <= data_.size() - (object - kData)) {
      store(object + kUStructSuper, super);
      store(object + kUStructPropertiesSize, properties);
    }
  }

  RawShared make_shared(const std::uintptr_t object) {
    const auto controller = allocate(0x10, 8);
    store(controller + kSharedControllerVtable, kImage + 0x00234560);
    store(controller + kSharedControllerStrong, std::int32_t{1});
    store(controller + kSharedControllerWeak, std::int32_t{1});
    return {object, controller};
  }

  void put_fstring(const std::uintptr_t destination, const std::u16string_view value) {
    if (value.empty()) {
      put_array(destination, 0, 0, 0);
      return;
    }
    const auto data = allocate((value.size() + 1) * sizeof(std::uint16_t), 2);
    for (std::size_t index = 0; index < value.size(); ++index) {
      store(data + index * sizeof(std::uint16_t),
            static_cast<std::uint16_t>(value[index]));
    }
    store(data + value.size() * sizeof(std::uint16_t), std::uint16_t{0});
    put_array(destination, data, static_cast<std::int32_t>(value.size() + 1),
              static_cast<std::int32_t>(value.size() + 1));
  }

  void put_array(const std::uintptr_t destination, const std::uintptr_t data,
                 const std::int32_t num, const std::int32_t capacity) {
    store(destination, RawArray{data, num, capacity});
  }

  void put_sparse(const std::uintptr_t destination, const std::uintptr_t elements,
                  const std::int32_t num, const std::int32_t capacity,
                  const std::uintptr_t bits, const std::int32_t bit_count,
                  const std::int32_t bit_capacity) {
    put_array(destination, elements, num, capacity);
    store(destination + kSparseSecondaryAllocation,
          bit_capacity > static_cast<std::int32_t>(
                             kSparseInlineAllocationWords * 32)
              ? bits
              : std::uintptr_t{0});
    store(destination + kSparseAllocationNum, bit_count);
    store(destination + kSparseAllocationMax, bit_capacity);
    if (bit_capacity <= static_cast<std::int32_t>(
                            kSparseInlineAllocationWords * 32) &&
        bit_capacity != 0) {
      std::array<std::uint32_t, kSparseInlineAllocationWords> inline_bits{};
      const auto words = (static_cast<std::size_t>(bit_capacity) + 31) / 32;
      for (std::size_t index = 0; index < words; ++index) {
        inline_bits[index] = load<std::uint32_t>(
            bits + index * sizeof(std::uint32_t));
      }
      for (std::size_t index = 0; index < inline_bits.size(); ++index) {
        store(destination + kSparseInlineAllocationFlags +
                  index * sizeof(std::uint32_t),
              inline_bits[index]);
      }
    }
    store(destination + kSparseFirstFree, std::int32_t{-1});
    store(destination + kSparseNumFree, std::int32_t{0});
  }

  static std::uint64_t fixture_xxh(const std::u16string_view value) noexcept {
    return xxh64({reinterpret_cast<const std::byte*>(value.data()),
                  value.size() * sizeof(char16_t)});
  }

  std::vector<std::byte> data_;
  std::vector<std::byte> pool_;
  std::vector<std::byte> static_names_;
  std::vector<std::byte> class_delegate_;
  std::vector<std::byte> graph_delegates_;
  std::size_t cursor_{};
  std::size_t name_cursor_{};
  std::uintptr_t name_block_{};
  bool valid_{true};
  std::int32_t next_object_index_{};
  std::uintptr_t meta_class_{};
  std::uintptr_t mod_package_{};
  std::uintptr_t engine_package_{};
  std::uintptr_t core_package_{};
  std::uintptr_t object_class_{};
  std::uintptr_t actor_class_{};
  std::uintptr_t mod_actor_class_{};
  std::uintptr_t class_descriptor_{};
  RawShared class_shared_{};
  std::uintptr_t module_{};
  RawShared module_shared_{};
  std::uintptr_t file_{};
  std::uintptr_t preprocessor_{};
  std::uintptr_t manager_{};
  std::uintptr_t descriptor_array_{};
  std::uintptr_t generated_statics_{};
  std::uintptr_t class_shared_address_{};
  std::uintptr_t has_statics_{};
  std::uintptr_t flag_bits_{};
  std::uintptr_t class_delegate_entry_{};
};

}  // namespace

bool target_frontend_raw_materializer_selftest_v1() noexcept {
  try {
    RawFixture fixture;
    if (!fixture.build()) return false;
    TargetFrontendSnapshot snapshot;
    if (fixture.snapshot(snapshot) != Error::ok || snapshot.epoch() != 7) return false;

    std::vector<FrontendFlag> flags;
    std::vector<std::string> specializations;
    std::vector<FrontendFNameComparison> names;
    std::vector<TargetFrontendRawFile> files;
    std::vector<FrontendGraphModule> chunks;
    std::vector<FrontendGraphModule> processed;
    std::vector<FrontendGraphModule> descriptors;
    FrontendNativeClassWitness native{};
    TargetFrontendNativeSuperRaw class_native{};
    FrontendClassFrame frame{};
    TargetFrontendGraphHookBindings bindings{};
    FrontendPreprocessorConfig hook_config{};
    if (materialize_graph_hook_bindings_v1(snapshot, bindings) != Error::ok ||
        bindings.class_analyze_bound || bindings.class_analyze_active_bindings != 0 ||
        bindings.process_chunks_bound || bindings.post_process_code_bound ||
        materialize_graph_hook_config_v1(snapshot, hook_config) != Error::ok ||
        hook_config.process_chunks_bound ||
        !hook_config.process_chunks_captures.empty() ||
        hook_config.post_process_code_bound ||
        !hook_config.post_process_code_captures.empty() ||
        materialize_preprocessor_flags_v1(snapshot, fixture.preprocessor(), flags) !=
            Error::ok ||
        flags.size() != 2 || flags[0].name != "RELEASE" || !flags[0].value ||
        flags[1].name != "TEST" || flags[1].value ||
        materialize_blueprint_specializations_v1(
            snapshot, fixture.manager(), specializations) != Error::ok ||
        specializations != std::vector<std::string>{"FName", "int32"} ||
        materialize_static_fnames_v1(snapshot, names) != Error::ok || names.size() != 1 ||
        names[0].spelling != "Gr\xc3\xb6\xc3\x9f\x65" ||
        materialize_preprocessor_files_v1(snapshot, fixture.preprocessor(), files) !=
            Error::ok ||
        files.size() != 1 || files[0].chunks.size() != 1 ||
        files[0].chunks[0].class_name != "AModActor" ||
        materialize_preprocessor_graph_v1(
            snapshot, fixture.preprocessor(), TargetFrontendGraphSource::chunk_content,
            chunks) != Error::ok ||
        materialize_preprocessor_graph_v1(
            snapshot, fixture.preprocessor(), TargetFrontendGraphSource::processed_code,
            processed) != Error::ok ||
        chunks.size() != 1 || chunks[0].sections.size() != 1 ||
        chunks[0].sections[0].code != processed[0].sections[0].code ||
        chunks[0].generated_declarations != "\n\nvoid Generated();" ||
        materialize_module_descriptor_graph_v1(
            snapshot, fixture.descriptor_array(), descriptors) != Error::ok ||
        descriptors.size() != 1 || descriptors[0].module_name != "Mods.Sample" ||
        materialize_uclass_witness_v1(
            snapshot, fixture.mod_actor_class(), "AModActor", 64, native) != Error::ok ||
        native.ancestry_paths !=
            std::vector<std::string>{"/Script/Mod.ModActor", "/Script/Engine.Actor",
                                     "/Script/CoreUObject.Object"} ||
        materialize_class_native_super_v1(
            snapshot, fixture.class_shared_address(), class_native) != Error::ok ||
        !class_native.present || class_native.witness.angelscript_type_name != "AActor" ||
        class_native.witness.property_offset != 96 ||
        class_native.witness.unreal_class_path != "/Script/Engine.Actor" ||
        materialize_class_analyze_frame_v1(
            snapshot, fixture.file(), fixture.generated_statics(),
            fixture.class_shared_address(), fixture.has_statics(), frame) != Error::ok ||
        frame.module_name != "Mods.Sample" || frame.name_space != "Example" ||
        frame.class_name != "AModActor" || !frame.has_statics ||
        frame.compose_onto_class != "AParent") {
      return false;
    }

    fixture.set_process_delegate_pointer(RawFixture::kData);
    TargetFrontendSnapshot bound_delegate;
    if (fixture.snapshot(bound_delegate) != Error::ok ||
        materialize_graph_hook_bindings_v1(bound_delegate, bindings) !=
            Error::unresolved_semantics) {
      return false;
    }
    fixture.set_process_delegate_pointer(0);

    fixture.set_class_delegate_bound(true);
    TargetFrontendSnapshot class_bound_delegate;
    if (fixture.snapshot(class_bound_delegate) != Error::ok ||
        materialize_graph_hook_bindings_v1(class_bound_delegate, bindings) != Error::ok ||
        !bindings.class_analyze_bound || bindings.class_analyze_active_bindings != 1 ||
        materialize_graph_hook_config_v1(class_bound_delegate, hook_config) != Error::ok ||
        !hook_config.class_analyze_bound) {
      return false;
    }
    fixture.set_class_delegate_bound(false);

    fixture.store(fixture.class_controller() + kSharedControllerStrong, std::int32_t{0});
    TargetFrontendSnapshot corrupt_owner;
    if (fixture.snapshot(corrupt_owner) != Error::ok ||
        materialize_class_analyze_frame_v1(
            corrupt_owner, fixture.file(), fixture.generated_statics(),
            fixture.class_shared_address(), fixture.has_statics(), frame) !=
            Error::invalid_shared_owner) {
      return false;
    }
    fixture.store(fixture.class_controller() + kSharedControllerStrong, std::int32_t{1});
    fixture.store(fixture.flag_bits(), std::uint32_t{0});
    TargetFrontendSnapshot corrupt_sparse;
    if (fixture.snapshot(corrupt_sparse) != Error::ok ||
        materialize_preprocessor_flags_v1(
            corrupt_sparse, fixture.preprocessor(), flags) != Error::invalid_container) {
      return false;
    }
    fixture.store(fixture.flag_bits(), std::uint32_t{3});
    fixture.store(fixture.mod_actor_class() + kUStructSuper, fixture.mod_actor_class());
    TargetFrontendSnapshot corrupt_cycle;
    if (fixture.snapshot(corrupt_cycle) != Error::ok ||
        materialize_uclass_witness_v1(
            corrupt_cycle, fixture.mod_actor_class(), "AModActor", 64, native) !=
            Error::cyclic_ownership) {
      return false;
    }

    const std::array<std::byte, 8> overlap_bytes{};
    const std::array overlap{
        TargetRawRegionInput{RawFixture::kData, overlap_bytes.data(), overlap_bytes.size(),
                             RegionKind::immutable_data},
        TargetRawRegionInput{RawFixture::kData + 4, overlap_bytes.data(), overlap_bytes.size(),
                             RegionKind::immutable_data},
    };
    TargetFrontendSnapshot rejected;
    return TargetFrontendSnapshot::create(
               RawFixture::kImage, kPeSizeOfImage, 1, overlap, rejected) ==
           Error::invalid_snapshot;
  } catch (...) {
    return false;
  }
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
