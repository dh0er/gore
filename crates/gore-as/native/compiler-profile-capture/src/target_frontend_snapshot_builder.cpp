#include "target_frontend_snapshot_builder.hpp"

#include "gore_as_capture/format.hpp"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <set>
#include <utility>
#include <vector>

namespace gore_as_capture::v1::instrumentation {
namespace {

using BuildError = TargetFrontendSnapshotBuildError;
using RegionKind = TargetRawRegionKind;

constexpr std::size_t kMaximumClaims = 16'384;
constexpr std::size_t kMaximumBytes = 128u * 1024u * 1024u;
constexpr std::size_t kMaximumItems = 1'000'000;
constexpr std::size_t kMaximumFiles = 65'536;
constexpr std::size_t kMaximumChunks = 1'000'000;
constexpr std::size_t kMaximumObjectDepth = 64;
constexpr std::size_t kMaximumTextCodeUnits = 8u * 1024u * 1024u;
constexpr std::int32_t kMaximumSharedReferences = 0x3fff'ffff;
constexpr std::uint32_t kFNamePoolRva = frontend_target_layout::fname_pool_rva;
constexpr std::size_t kFNamePoolBlocks = 0x10;
constexpr std::uint32_t kFNameBlockShift = 16;
constexpr std::uint32_t kFNameOffsetMask = 0xffff;
constexpr std::uint32_t kFNameEntryStride = 2;
constexpr std::uint32_t kFNameHeaderLengthShift = 6;
constexpr std::uint16_t kFNameWideMask = 1;
constexpr std::uint32_t kMaximumFNameBlocks = 1u << 13;
constexpr std::size_t kSparseInlineAllocationFlags = 0x10;
constexpr std::size_t kSparseSecondaryAllocation = 0x20;
constexpr std::size_t kSparseAllocationNum = 0x28;
constexpr std::size_t kSparseAllocationMax = 0x2c;
constexpr std::size_t kSparseFirstFree = 0x30;
constexpr std::size_t kSparseNumFree = 0x34;
constexpr std::size_t kSparseInlineAllocationWords = 4;
constexpr std::size_t kMapFlagStride = 0x20;
constexpr std::size_t kSetStringStride = 0x18;
constexpr std::size_t kFileStride = 0xc8;
constexpr std::size_t kFileModule = 0x00;
constexpr std::size_t kFileAbsolute = 0x20;
constexpr std::size_t kFileRelative = 0x30;
constexpr std::size_t kFileRaw = 0x40;
constexpr std::size_t kFileChunkBlocks = 0x50;
constexpr std::size_t kFileChunkCount = 0x60;
constexpr std::size_t kFileProcessed = 0x68;
constexpr std::size_t kFileGenerated = 0x78;
constexpr std::size_t kChunkStride = 0x90;
constexpr std::size_t kChunkElementsPerBlock = 0x71;
constexpr std::size_t kChunkContent = 0x08;
constexpr std::size_t kChunkComment = 0x18;
constexpr std::size_t kChunkClassDescriptor = 0x58;
constexpr std::size_t kChunkNamespace = 0x68;
constexpr std::size_t kModuleBytes = 0x28;
constexpr std::size_t kModuleName = 0x00;
constexpr std::size_t kModuleCode = 0x10;
constexpr std::size_t kCodeSectionStride = 0x38;
constexpr std::size_t kCodeSectionRelative = 0x00;
constexpr std::size_t kCodeSectionAbsolute = 0x10;
constexpr std::size_t kCodeSectionCode = 0x20;
constexpr std::size_t kClassDescriptorBytes = 0x119;
constexpr std::size_t kClassName = 0x00;
constexpr std::size_t kClassSuperName = 0x10;
constexpr std::size_t kClassCodeSuper = 0x20;
constexpr std::size_t kClassSuperIsCode = 0x28;
constexpr std::size_t kClassCompose = 0xf8;
constexpr std::size_t kClassNamespace = 0x108;
constexpr std::size_t kUObjectBytes = 0x28;
constexpr std::size_t kUObjectVtable = 0x00;
constexpr std::size_t kUObjectInternalIndex = 0x0c;
constexpr std::size_t kUObjectClass = 0x10;
constexpr std::size_t kUObjectName = 0x18;
constexpr std::size_t kUObjectOuter = 0x20;
constexpr std::size_t kUClassBytes = 0x5c;
constexpr std::size_t kUClassSuper = 0x40;
constexpr std::size_t kSettingsBytes = 0x76;
constexpr std::size_t kPreprocessorBytes = 0x108;
constexpr std::size_t kManagerBytes = 0x4d8;

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

struct Claim final {
  std::uintptr_t address{};
  std::size_t bytes{};
  RegionKind kind{};
  std::vector<std::byte> first_copy;
};

struct MergedRegion final {
  std::uintptr_t address{};
  std::size_t bytes{};
  RegionKind kind{};
  std::vector<std::byte> stable_copy;
};

bool add_address(
    const std::uintptr_t base,
    const std::size_t offset,
    std::uintptr_t& result) noexcept {
  if (base > std::numeric_limits<std::uintptr_t>::max() - offset) return false;
  result = base + offset;
  return true;
}

bool multiply_size(
    const std::size_t left,
    const std::size_t right,
    std::size_t& result) noexcept {
  if (left != 0 && right > std::numeric_limits<std::size_t>::max() / left) return false;
  result = left * right;
  return true;
}

bool readable_protection(const DWORD protection) noexcept {
  switch (protection & 0xffu) {
    case PAGE_READONLY:
    case PAGE_READWRITE:
    case PAGE_WRITECOPY:
    case PAGE_EXECUTE_READ:
    case PAGE_EXECUTE_READWRITE:
    case PAGE_EXECUTE_WRITECOPY:
      return true;
    default:
      return false;
  }
}

bool current_process_read(
    const std::uintptr_t address,
    const std::span<std::byte> output) noexcept {
  if (address == 0 || output.empty() ||
      address > std::numeric_limits<std::uintptr_t>::max() - output.size()) {
    return false;
  }
  auto cursor = address;
  const auto end = address + output.size();
  while (cursor < end) {
    MEMORY_BASIC_INFORMATION region{};
    if (VirtualQuery(reinterpret_cast<const void*>(cursor), &region, sizeof(region)) !=
            sizeof(region) ||
        region.State != MEM_COMMIT || (region.Protect & PAGE_GUARD) != 0 ||
        !readable_protection(region.Protect)) {
      return false;
    }
    const auto base = reinterpret_cast<std::uintptr_t>(region.BaseAddress);
    if (base > std::numeric_limits<std::uintptr_t>::max() - region.RegionSize) return false;
    const auto next = base + region.RegionSize;
    if (cursor < base || next <= cursor) return false;
    cursor = std::min(end, next);
  }
  __try {
    std::memcpy(output.data(), reinterpret_cast<const void*>(address), output.size());
    return true;
  } __except (EXCEPTION_EXECUTE_HANDLER) {
    return false;
  }
}

class Builder final {
 public:
  Builder(const std::uintptr_t image, const std::uint64_t epoch) noexcept
      : image_(image), epoch_(epoch) {}

  BuildError build(
      const TargetFrontendSnapshotRoots& roots,
      TargetFrontendSnapshot& output) {
    auto status = validate_roots(roots);
    if (status != BuildError::ok) return status;
    switch (roots.phase) {
      case TargetFrontendSnapshotPhase::configuration:
        status = configuration(roots.manager, roots.preprocessor);
        break;
      case TargetFrontendSnapshotPhase::module_descriptors:
        status = descriptor_graph(roots.descriptor_array);
        break;
      case TargetFrontendSnapshotPhase::class_analyze:
        status = class_analyze(roots);
        break;
      case TargetFrontendSnapshotPhase::native_class:
        status = uclass_chain(roots.uclass);
        break;
      case TargetFrontendSnapshotPhase::hook_bindings:
        status = hook_bindings();
        break;
      case TargetFrontendSnapshotPhase::settings_configuration:
        status = settings_configuration(roots.manager);
        break;
      default:
        return BuildError::invalid_argument;
    }
    if (status != BuildError::ok) return status;
    return finish(output);
  }

 private:
  BuildError validate_roots(const TargetFrontendSnapshotRoots& roots) const noexcept {
    const auto no_config = roots.manager == 0 && roots.preprocessor == 0;
    const auto no_descriptor = roots.descriptor_array == 0;
    const auto no_class = roots.file == 0 && roots.generated_statics_fstring == 0 &&
                          roots.class_descriptor_shared == 0 && roots.has_statics == 0;
    const auto no_uclass = roots.uclass == 0;
    switch (roots.phase) {
      case TargetFrontendSnapshotPhase::configuration:
        return roots.manager != 0 && roots.preprocessor != 0 && no_descriptor && no_class &&
                       no_uclass
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      case TargetFrontendSnapshotPhase::module_descriptors:
        return no_config && roots.descriptor_array != 0 && no_class && no_uclass
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      case TargetFrontendSnapshotPhase::class_analyze:
        return no_config && no_descriptor && roots.file != 0 &&
                       roots.generated_statics_fstring != 0 &&
                       roots.class_descriptor_shared != 0 && roots.has_statics != 0 &&
                       no_uclass
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      case TargetFrontendSnapshotPhase::native_class:
        return no_config && no_descriptor && no_class && roots.uclass != 0
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      case TargetFrontendSnapshotPhase::hook_bindings:
        return no_config && no_descriptor && no_class && no_uclass
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      case TargetFrontendSnapshotPhase::settings_configuration:
        return roots.manager != 0 && roots.preprocessor == 0 && no_descriptor && no_class &&
                       no_uclass
                   ? BuildError::ok
                   : BuildError::invalid_argument;
      default:
        return BuildError::invalid_argument;
    }
  }

  bool is_image(const std::uintptr_t address, const std::size_t bytes) const noexcept {
    return address >= image_ && address - image_ <= kPeSizeOfImage &&
           bytes <= kPeSizeOfImage - (address - image_);
  }

  BuildError claim(
      const std::uintptr_t address,
      const std::size_t bytes,
      const RegionKind kind,
      std::vector<std::byte>* const copy = nullptr) {
    if (address == 0 || bytes == 0 ||
        address > std::numeric_limits<std::uintptr_t>::max() - bytes) {
      return BuildError::address_overflow;
    }
    if ((kind == RegionKind::primary_image) != is_image(address, bytes)) {
      return BuildError::wrong_ownership_region;
    }
    // Container backing allocations are claimed before their typed children. Reuse that first
    // immutable view for every covered child instead of counting/copying overlapping bytes again;
    // this both preserves one epoch and keeps the 128-MiB cap on unique ownership extents.
    const auto covered = std::find_if(claims_.begin(), claims_.end(), [&](const auto& value) {
      return value.kind == kind && address >= value.address &&
             address - value.address <= value.bytes &&
             bytes <= value.bytes - (address - value.address);
    });
    if (covered != claims_.end()) {
      if (copy != nullptr) {
        const auto offset = address - covered->address;
        copy->assign(covered->first_copy.begin() + offset,
                     covered->first_copy.begin() + offset + bytes);
      }
      return BuildError::ok;
    }
    if (claims_.size() == kMaximumClaims || bytes > kMaximumBytes ||
        claimed_bytes_ > kMaximumBytes - bytes) {
      return BuildError::limit_exceeded;
    }
    Claim value{address, bytes, kind, std::vector<std::byte>(bytes)};
    if (!current_process_read(address, value.first_copy)) return BuildError::unreadable_range;
    if (copy != nullptr) *copy = value.first_copy;
    claimed_bytes_ += bytes;
    claims_.push_back(std::move(value));
    return BuildError::ok;
  }

  template <typename Type>
  BuildError read(
      const std::uintptr_t address,
      const RegionKind kind,
      Type& value) {
    static_assert(std::is_trivially_copyable_v<Type>);
    std::vector<std::byte> copy;
    const auto status = claim(address, sizeof(Type), kind, &copy);
    if (status == BuildError::ok) std::memcpy(&value, copy.data(), sizeof(value));
    return status;
  }

  BuildError array(
      const std::uintptr_t address,
      const std::size_t stride,
      const std::size_t maximum,
      RawArray& value,
      const RegionKind header_kind = RegionKind::immutable_data) {
    auto status = read(address, header_kind, value);
    if (status != BuildError::ok) return status;
    if (stride == 0 || value.num < 0 || value.capacity < 0 || value.num > value.capacity ||
        static_cast<std::size_t>(value.capacity) > maximum) {
      return BuildError::invalid_container;
    }
    if (value.capacity == 0) {
      return value.num == 0 && value.data == 0 ? BuildError::ok
                                               : BuildError::invalid_container;
    }
    std::size_t bytes = 0;
    if (value.data == 0 || !multiply_size(static_cast<std::size_t>(value.capacity), stride,
                                          bytes)) {
      return BuildError::invalid_container;
    }
    return claim(value.data, bytes, RegionKind::immutable_data);
  }

  BuildError fstring(const std::uintptr_t address) {
    RawArray value{};
    auto status = array(address, sizeof(std::uint16_t), kMaximumTextCodeUnits + 1, value);
    if (status != BuildError::ok || value.num == 0) return status;
    std::uint16_t terminator = 1;
    status = read(value.data + (static_cast<std::size_t>(value.num) - 1) *
                                   sizeof(std::uint16_t),
                  RegionKind::immutable_data, terminator);
    if (status != BuildError::ok) return status;
    return terminator == 0 ? BuildError::ok : BuildError::target_layout_drift;
  }

  BuildError shared(
      const std::uintptr_t address,
      const bool nullable,
      RawShared& value) {
    auto status = read(address, RegionKind::immutable_data, value);
    if (status != BuildError::ok) return status;
    if (value.object == 0 || value.controller == 0) {
      return nullable && value.object == 0 && value.controller == 0
                 ? BuildError::ok
                 : BuildError::invalid_shared_owner;
    }
    std::array<std::byte, 0x10> controller{};
    std::vector<std::byte> copy;
    status = claim(value.controller, controller.size(), RegionKind::immutable_data, &copy);
    if (status != BuildError::ok) return status;
    std::memcpy(controller.data(), copy.data(), copy.size());
    std::uintptr_t vtable = 0;
    std::int32_t strong = 0;
    std::int32_t weak = 0;
    std::memcpy(&vtable, controller.data(), sizeof(vtable));
    std::memcpy(&strong, controller.data() + 8, sizeof(strong));
    std::memcpy(&weak, controller.data() + 12, sizeof(weak));
    return is_image(vtable, 1) && strong > 0 && weak > 0 &&
                   strong <= kMaximumSharedReferences && weak <= kMaximumSharedReferences
               ? BuildError::ok
               : BuildError::invalid_shared_owner;
  }

  BuildError sparse_slots(
      const std::uintptr_t address,
      const std::size_t stride,
      const std::size_t maximum,
      std::vector<std::uintptr_t>& slots) {
    RawArray elements{};
    auto status = array(address, stride, maximum, elements);
    if (status != BuildError::ok) return status;
    std::uintptr_t secondary_allocation = 0;
    std::int32_t allocation_num = 0;
    std::int32_t allocation_capacity = 0;
    std::int32_t first_free = 0;
    std::int32_t num_free = 0;
    if (read(address + kSparseSecondaryAllocation, RegionKind::immutable_data,
             secondary_allocation) != BuildError::ok ||
        read(address + kSparseAllocationNum, RegionKind::immutable_data,
             allocation_num) != BuildError::ok ||
        read(address + kSparseAllocationMax, RegionKind::immutable_data,
             allocation_capacity) != BuildError::ok ||
        read(address + kSparseFirstFree, RegionKind::immutable_data, first_free) !=
            BuildError::ok ||
        read(address + kSparseNumFree, RegionKind::immutable_data, num_free) !=
            BuildError::ok ||
        allocation_num < 0 || allocation_capacity < 0 ||
        allocation_num > allocation_capacity || allocation_num != elements.num ||
        static_cast<std::size_t>(allocation_capacity) > maximum ||
        num_free < 0 || num_free > elements.num ||
        (num_free == 0 ? first_free != -1
                       : (first_free < 0 || first_free >= elements.num))) {
      return BuildError::invalid_container;
    }
    slots.clear();
    if (elements.num == 0) {
      return allocation_capacity == 0 && secondary_allocation == 0
                 ? BuildError::ok
                 : BuildError::invalid_container;
    }
    const auto capacity_words =
        (static_cast<std::size_t>(allocation_capacity) + 31) / 32;
    const auto allocation_data =
        capacity_words <= kSparseInlineAllocationWords
            ? address + kSparseInlineAllocationFlags
            : secondary_allocation;
    if (capacity_words == 0 ||
        (capacity_words <= kSparseInlineAllocationWords && secondary_allocation != 0) ||
        (capacity_words > kSparseInlineAllocationWords && secondary_allocation == 0)) {
      return BuildError::invalid_container;
    }
    std::vector<std::byte> copy;
    status = claim(allocation_data, capacity_words * sizeof(std::uint32_t),
                   RegionKind::immutable_data, &copy);
    if (status != BuildError::ok) return status;
    for (std::int32_t index = 0; index < elements.num; ++index) {
      std::uint32_t word = 0;
      std::memcpy(&word, copy.data() + static_cast<std::size_t>(index / 32) * sizeof(word),
                  sizeof(word));
      if ((word & (1u << (static_cast<std::uint32_t>(index) & 31u))) != 0) {
        slots.push_back(elements.data + static_cast<std::size_t>(index) * stride);
      }
    }
    return slots.size() == static_cast<std::size_t>(elements.num - num_free)
               ? BuildError::ok
               : BuildError::invalid_container;
  }

  BuildError fname(const TargetRawFName raw) {
    const auto block_index = raw.comparison_index >> kFNameBlockShift;
    if (block_index >= kMaximumFNameBlocks) return BuildError::invalid_fname;
    std::uintptr_t block = 0;
    const auto pointer = image_ + kFNamePoolRva + kFNamePoolBlocks +
                         static_cast<std::size_t>(block_index) * sizeof(block);
    auto status = read(pointer, RegionKind::primary_image, block);
    if (status != BuildError::ok || block == 0 || is_image(block, 1)) {
      return BuildError::invalid_fname;
    }
    const auto offset = (raw.comparison_index & kFNameOffsetMask) * kFNameEntryStride;
    std::uintptr_t entry = 0;
    if (!add_address(block, offset, entry)) return BuildError::address_overflow;
    std::uint16_t header = 0;
    status = read(entry, RegionKind::immutable_data, header);
    const auto length = static_cast<std::size_t>(header >> kFNameHeaderLengthShift);
    if (status != BuildError::ok || length == 0 || length > 1023) {
      return BuildError::invalid_fname;
    }
    const auto width = (header & kFNameWideMask) != 0 ? 2u : 1u;
    return claim(entry, sizeof(header) + length * width, RegionKind::immutable_data);
  }

  BuildError static_fnames() {
    RawArray names{};
    const auto header = image_ + frontend_target_layout::static_names_rva;
    auto status = array(header, sizeof(TargetRawFName), kMaximumItems, names,
                        RegionKind::primary_image);
    if (status != BuildError::ok) return status;
    for (std::int32_t index = 0; index < names.num; ++index) {
      TargetRawFName name{};
      status = read(names.data + static_cast<std::size_t>(index) * sizeof(name),
                    RegionKind::immutable_data, name);
      if (status != BuildError::ok || (status = fname(name)) != BuildError::ok) return status;
    }
    return BuildError::ok;
  }

  BuildError object_path(const std::uintptr_t object) {
    std::set<std::uintptr_t> seen;
    auto cursor = object;
    std::size_t depth = 0;
    while (cursor != 0) {
      if (depth++ == kMaximumObjectDepth || !seen.insert(cursor).second) {
        return BuildError::cyclic_ownership;
      }
      std::vector<std::byte> copy;
      auto status = claim(cursor, kUObjectBytes, RegionKind::immutable_data, &copy);
      if (status != BuildError::ok) return status;
      std::uintptr_t vtable = 0;
      std::int32_t internal_index = -1;
      std::uintptr_t object_class = 0;
      TargetRawFName name{};
      std::uintptr_t outer = 0;
      std::memcpy(&vtable, copy.data() + kUObjectVtable, sizeof(vtable));
      std::memcpy(&internal_index, copy.data() + kUObjectInternalIndex,
                  sizeof(internal_index));
      std::memcpy(&object_class, copy.data() + kUObjectClass, sizeof(object_class));
      std::memcpy(&name, copy.data() + kUObjectName, sizeof(name));
      std::memcpy(&outer, copy.data() + kUObjectOuter, sizeof(outer));
      if (!is_image(vtable, 1) || internal_index < 0 || object_class == 0 ||
          is_image(object_class, 1)) {
        return BuildError::target_layout_drift;
      }
      // is_data_address(object_class) is a materializer invariant. This exact UObject header is
      // the typed witness; its outgoing edges are intentionally not followed from this role.
      status = claim(object_class, kUObjectBytes, RegionKind::immutable_data);
      if (status != BuildError::ok || (status = fname(name)) != BuildError::ok) return status;
      cursor = outer;
    }
    return depth == 2 ? BuildError::ok : BuildError::target_layout_drift;
  }

  BuildError uclass_chain(const std::uintptr_t root) {
    std::set<std::uintptr_t> seen;
    auto cursor = root;
    std::size_t depth = 0;
    while (cursor != 0) {
      if (depth++ == kMaximumObjectDepth || !seen.insert(cursor).second) {
        return BuildError::cyclic_ownership;
      }
      std::vector<std::byte> copy;
      auto status = claim(cursor, kUClassBytes, RegionKind::immutable_data, &copy);
      if (status != BuildError::ok || (status = object_path(cursor)) != BuildError::ok) {
        return status;
      }
      std::uintptr_t object_class = 0;
      std::memcpy(&object_class, copy.data() + kUObjectClass, sizeof(object_class));
      if (object_class == 0 || is_image(object_class, 1) ||
          (status = object_path(object_class)) != BuildError::ok) {
        return status != BuildError::ok ? status : BuildError::target_layout_drift;
      }
      std::memcpy(&cursor, copy.data() + kUClassSuper, sizeof(cursor));
    }
    return depth != 0 ? BuildError::ok : BuildError::invalid_argument;
  }

  BuildError module_name(const std::uintptr_t module) {
    auto status = claim(module, kModuleBytes, RegionKind::immutable_data);
    return status == BuildError::ok ? fstring(module + kModuleName) : status;
  }

  BuildError class_descriptor(const std::uintptr_t descriptor) {
    auto status = claim(descriptor, kClassDescriptorBytes, RegionKind::immutable_data);
    if (status != BuildError::ok) return status;
    status = fstring(descriptor + kClassName);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container ? BuildError::class_name_container
                                                     : status;
    }
    status = fstring(descriptor + kClassSuperName);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container
                 ? BuildError::class_super_name_container
                 : status;
    }
    status = fstring(descriptor + kClassCompose);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container ? BuildError::class_compose_container
                                                     : status;
    }
    std::uint8_t native_super = 0;
    status = read(descriptor + kClassSuperIsCode, RegionKind::immutable_data, native_super);
    if (status != BuildError::ok || native_super > 1) {
      return status != BuildError::ok ? status : BuildError::target_layout_drift;
    }
    if (native_super != 0) {
      std::uintptr_t uclass = 0;
      status = read(descriptor + kClassCodeSuper, RegionKind::immutable_data, uclass);
      if (status != BuildError::ok || uclass == 0 || is_image(uclass, 1) ||
          (status = uclass_chain(uclass)) != BuildError::ok) {
        return status != BuildError::ok ? status : BuildError::target_layout_drift;
      }
    }
    std::uint8_t has_namespace = 0;
    status = read(descriptor + 0x118, RegionKind::immutable_data, has_namespace);
    if (status != BuildError::ok || has_namespace > 1) {
      return status != BuildError::ok ? status : BuildError::target_layout_drift;
    }
    if (has_namespace == 0) return BuildError::ok;
    status = fstring(descriptor + kClassNamespace);
    return status == BuildError::invalid_container ? BuildError::class_namespace_container
                                                    : status;
  }

  BuildError chunk(const std::uintptr_t address) {
    auto status = claim(address, kChunkStride, RegionKind::immutable_data);
    if (status != BuildError::ok ||
        (status = fstring(address + kChunkContent)) != BuildError::ok ||
        (status = fstring(address + kChunkComment)) != BuildError::ok) {
      return status;
    }
    std::uint8_t has_namespace = 0;
    status = read(address + 0x78, RegionKind::immutable_data, has_namespace);
    if (status != BuildError::ok || has_namespace > 1) {
      return status != BuildError::ok ? status : BuildError::target_layout_drift;
    }
    if (has_namespace != 0 &&
        (status = fstring(address + kChunkNamespace)) != BuildError::ok) {
      return status;
    }
    RawShared descriptor{};
    status = shared(address + kChunkClassDescriptor, true, descriptor);
    return status == BuildError::ok && descriptor.object != 0
               ? class_descriptor(descriptor.object)
               : status;
  }

  BuildError file(const std::uintptr_t address) {
    auto status = claim(address, kFileStride, RegionKind::immutable_data);
    if (status != BuildError::ok) return status;
    RawShared module{};
    status = shared(address + kFileModule, false, module);
    if (status != BuildError::ok) return status;
    status = module_name(module.object);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container ? BuildError::file_module_name_container
                                                     : status;
    }
    for (const auto [offset, container_error] : {
             std::pair{kFileAbsolute, BuildError::file_absolute_path_container},
             std::pair{kFileRelative, BuildError::file_relative_path_container},
             std::pair{kFileRaw, BuildError::file_raw_code_container},
             std::pair{kFileProcessed, BuildError::file_processed_code_container}}) {
      status = fstring(address + offset);
      if (status != BuildError::ok) {
        return status == BuildError::invalid_container ? container_error : status;
      }
    }
    RawArray generated{};
    status = array(address + kFileGenerated, sizeof(RawArray), kMaximumItems, generated);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container
                 ? BuildError::file_generated_array_container
                 : status;
    }
    for (std::int32_t index = 0; index < generated.num; ++index) {
      status = fstring(generated.data + static_cast<std::size_t>(index) * sizeof(RawArray));
      if (status != BuildError::ok) {
        return status == BuildError::invalid_container
                   ? BuildError::file_generated_string_container
                   : status;
      }
    }
    RawArray blocks{};
    status = array(address + kFileChunkBlocks, sizeof(std::uintptr_t),
                   (kMaximumChunks + kChunkElementsPerBlock - 1) / kChunkElementsPerBlock,
                   blocks);
    if (status != BuildError::ok) {
      return status == BuildError::invalid_container
                 ? BuildError::file_chunk_blocks_container
                 : status;
    }
    std::int32_t count = 0;
    status = read(address + kFileChunkCount, RegionKind::immutable_data, count);
    if (status != BuildError::ok || count < 0 ||
        static_cast<std::size_t>(count) > kMaximumChunks) {
      return BuildError::file_chunk_count_container;
    }
    const auto required = (static_cast<std::size_t>(count) + kChunkElementsPerBlock - 1) /
                          kChunkElementsPerBlock;
    if (static_cast<std::size_t>(blocks.num) != required) {
      return BuildError::file_chunk_count_container;
    }
    for (std::int32_t index = 0; index < count; ++index) {
      const auto block_index = static_cast<std::size_t>(index) / kChunkElementsPerBlock;
      const auto element = static_cast<std::size_t>(index) % kChunkElementsPerBlock;
      std::uintptr_t block = 0;
      status = read(blocks.data + block_index * sizeof(block), RegionKind::immutable_data,
                    block);
      if (status != BuildError::ok || block == 0 || is_image(block, 1)) {
        return BuildError::file_chunk_count_container;
      }
      status = chunk(block + element * kChunkStride);
      if (status != BuildError::ok) {
        return status == BuildError::invalid_container ? BuildError::file_chunk_container
                                                       : status;
      }
    }
    return BuildError::ok;
  }

  BuildError preprocessor(const std::uintptr_t address) {
    auto status = claim(address, kPreprocessorBytes, RegionKind::immutable_data);
    if (status != BuildError::ok) return status;
    std::vector<std::uintptr_t> slots;
    status = sparse_slots(address, kMapFlagStride, kMaximumItems, slots);
    if (status != BuildError::ok) return status;
    for (const auto slot : slots) {
      if ((status = fstring(slot)) != BuildError::ok) return status;
    }
    RawArray files{};
    status = array(address + frontend_target_layout::preprocessor_files, kFileStride,
                   kMaximumFiles, files);
    if (status != BuildError::ok) return status;
    for (std::int32_t index = 0; index < files.num; ++index) {
      status = file(files.data + static_cast<std::size_t>(index) * kFileStride);
      if (status != BuildError::ok) return status;
    }
    return BuildError::ok;
  }

  BuildError configuration(
      const std::uintptr_t manager,
      const std::uintptr_t preprocessor_address) {
    std::vector<std::byte> manager_copy;
    auto status = claim(manager, kManagerBytes, RegionKind::immutable_data, &manager_copy);
    if (status != BuildError::ok) return status;
    std::uintptr_t settings = 0;
    std::memcpy(&settings,
                manager_copy.data() + frontend_target_layout::manager_settings,
                sizeof(settings));
    if (settings == 0 || is_image(settings, 1)) return BuildError::target_layout_drift;
    status = claim(settings, kSettingsBytes, RegionKind::immutable_data);
    if (status != BuildError::ok) return status;
    RawArray settings_flags{};
    status = array(settings + frontend_target_layout::settings_preprocessor_flags,
                   sizeof(RawArray), kMaximumItems, settings_flags);
    if (status != BuildError::ok) return BuildError::configuration_settings_flags;
    for (std::int32_t index = 0; index < settings_flags.num; ++index) {
      if ((status = fstring(
               settings_flags.data + static_cast<std::size_t>(index) *
                                         sizeof(RawArray))) != BuildError::ok) {
        return BuildError::configuration_settings_flags;
      }
    }
    std::vector<std::uintptr_t> slots;
    status = sparse_slots(
        manager + frontend_target_layout::manager_blueprint_specializations,
        kSetStringStride, kMaximumItems, slots);
    if (status != BuildError::ok) {
      return BuildError::configuration_blueprint_specializations;
    }
    for (const auto slot : slots) {
      if ((status = fstring(slot)) != BuildError::ok) {
        return BuildError::configuration_blueprint_specializations;
      }
    }
    if ((status = preprocessor(preprocessor_address)) != BuildError::ok) {
      return BuildError::configuration_preprocessor;
    }
    if ((status = static_fnames()) != BuildError::ok) {
      return BuildError::configuration_static_fnames;
    }
    if ((status = hook_bindings()) != BuildError::ok) {
      return BuildError::configuration_hook_bindings;
    }
    if ((status = claim(image_ + frontend_target_layout::automatic_imports_rva, 1,
                        RegionKind::primary_image)) != BuildError::ok) {
      return status;
    }
    return claim(image_ + frontend_target_layout::use_editor_scripts_rva, 1,
                 RegionKind::primary_image);
  }

  BuildError settings_configuration(const std::uintptr_t manager) {
    std::vector<std::byte> manager_copy;
    auto status = claim(manager, kManagerBytes, RegionKind::immutable_data, &manager_copy);
    if (status != BuildError::ok) return status;
    std::uintptr_t settings = 0;
    std::memcpy(&settings,
                manager_copy.data() + frontend_target_layout::manager_settings,
                sizeof(settings));
    if (settings == 0 || is_image(settings, 1)) return BuildError::target_layout_drift;
    status = claim(settings, kSettingsBytes, RegionKind::immutable_data);
    if (status != BuildError::ok) return status;
    RawArray settings_flags{};
    status = array(settings + frontend_target_layout::settings_preprocessor_flags,
                   sizeof(RawArray), kMaximumItems, settings_flags);
    if (status != BuildError::ok) return status;
    for (std::int32_t index = 0; index < settings_flags.num; ++index) {
      if ((status = fstring(
               settings_flags.data + static_cast<std::size_t>(index) *
                                         sizeof(RawArray))) != BuildError::ok) {
        return status;
      }
    }
    std::vector<std::uintptr_t> slots;
    status = sparse_slots(
        manager + frontend_target_layout::manager_blueprint_specializations,
        kSetStringStride, kMaximumItems, slots);
    if (status != BuildError::ok) return status;
    for (const auto slot : slots) {
      if ((status = fstring(slot)) != BuildError::ok) return status;
    }
    if ((status = static_fnames()) != BuildError::ok ||
        (status = hook_bindings()) != BuildError::ok ||
        (status = claim(image_ + frontend_target_layout::automatic_imports_rva, 1,
                        RegionKind::primary_image)) != BuildError::ok) {
      return status;
    }
    return claim(image_ + frontend_target_layout::use_editor_scripts_rva, 1,
                 RegionKind::primary_image);
  }

  BuildError hook_bindings() {
    // The generation-selected GetOnClassAnalyze accessor returns the class-analyze delegate. Its
    // multicast invocation list is a TArray of exact 16-byte delegate pairs;
    // the three callsites' Broadcast implementations use the same trailing
    // compaction/broadcast counters. Copy only that typed object and its owned
    // allocation, never adjacent image/heap bytes.
    RawArray class_delegate{};
    const auto class_object =
        image_ + frontend_target_layout::class_analyze_delegate_rva;
    auto status = array(class_object, 0x10, kMaximumItems, class_delegate,
                        RegionKind::primary_image);
    if (status != BuildError::ok) return status;
    status = claim(class_object, 0x18, RegionKind::primary_image);
    if (status != BuildError::ok) return status;
    for (const auto rva : {frontend_target_layout::process_chunks_delegate_rva,
                           frontend_target_layout::post_process_code_delegate_rva}) {
      status = claim(image_ + rva, 0x18, RegionKind::primary_image);
      if (status != BuildError::ok) return status;
    }
    return BuildError::ok;
  }

  BuildError descriptor_graph(const std::uintptr_t descriptor_array) {
    RawArray descriptors{};
    auto status = array(descriptor_array, sizeof(RawShared), kMaximumFiles, descriptors);
    if (status != BuildError::ok) return status;
    for (std::int32_t index = 0; index < descriptors.num; ++index) {
      RawShared module{};
      status = shared(descriptors.data + static_cast<std::size_t>(index) * sizeof(module),
                      false, module);
      if (status != BuildError::ok ||
          (status = module_name(module.object)) != BuildError::ok) {
        return status;
      }
      RawArray sections{};
      status = array(module.object + kModuleCode, kCodeSectionStride, kMaximumItems,
                     sections);
      if (status != BuildError::ok) return status;
      for (std::int32_t section = 0; section < sections.num; ++section) {
        const auto entry = sections.data + static_cast<std::size_t>(section) *
                                               kCodeSectionStride;
        status = claim(entry, kCodeSectionStride, RegionKind::immutable_data);
        if (status != BuildError::ok) return status;
        for (const auto offset : {kCodeSectionRelative, kCodeSectionAbsolute,
                                  kCodeSectionCode}) {
          if ((status = fstring(entry + offset)) != BuildError::ok) return status;
        }
      }
    }
    return BuildError::ok;
  }

  BuildError class_analyze(const TargetFrontendSnapshotRoots& roots) {
    auto status = file(roots.file);
    if (status != BuildError::ok ||
        (status = fstring(roots.generated_statics_fstring)) != BuildError::ok) {
      return status;
    }
    RawShared descriptor{};
    status = shared(roots.class_descriptor_shared, false, descriptor);
    if (status != BuildError::ok ||
        (status = class_descriptor(descriptor.object)) != BuildError::ok) {
      return status;
    }
    return claim(roots.has_statics, 1, RegionKind::immutable_data);
  }

  BuildError finish(TargetFrontendSnapshot& output) {
    std::sort(claims_.begin(), claims_.end(), [](const Claim& left, const Claim& right) {
      return left.address < right.address ||
             (left.address == right.address && left.kind < right.kind);
    });
    std::vector<MergedRegion> merged;
    for (const auto& claim_value : claims_) {
      if (!merged.empty()) {
        auto& previous = merged.back();
        const auto previous_end = previous.address + previous.bytes;
        if (claim_value.address < previous_end && previous.kind != claim_value.kind) {
          return BuildError::wrong_ownership_region;
        }
        if (claim_value.kind == previous.kind && claim_value.address <= previous_end) {
          const auto claim_end = claim_value.address + claim_value.bytes;
          previous.bytes = std::max(previous_end, claim_end) - previous.address;
          continue;
        }
      }
      merged.push_back({claim_value.address, claim_value.bytes, claim_value.kind, {}});
    }
    std::size_t total = 0;
    for (auto& region : merged) {
      if (region.bytes > kMaximumBytes || total > kMaximumBytes - region.bytes) {
        return BuildError::limit_exceeded;
      }
      region.stable_copy.resize(region.bytes);
      if (!current_process_read(region.address, region.stable_copy)) {
        return BuildError::unreadable_range;
      }
      total += region.bytes;
    }
    for (const auto& claim_value : claims_) {
      const auto found = std::find_if(merged.begin(), merged.end(), [&](const auto& region) {
        return region.kind == claim_value.kind && claim_value.address >= region.address &&
               claim_value.address - region.address <= region.bytes &&
               claim_value.bytes <= region.bytes - (claim_value.address - region.address);
      });
      if (found == merged.end() ||
          std::memcmp(found->stable_copy.data() + (claim_value.address - found->address),
                      claim_value.first_copy.data(), claim_value.bytes) != 0) {
        return BuildError::lifetime_drift;
      }
    }
    // A second final copy closes mutation during the stable-copy pass itself.
    std::vector<TargetRawRegionInput> inputs;
    inputs.reserve(merged.size());
    for (auto& region : merged) {
      std::vector<std::byte> second(region.bytes);
      if (!current_process_read(region.address, second) || second != region.stable_copy) {
        return BuildError::lifetime_drift;
      }
      inputs.push_back({region.address, region.stable_copy.data(), region.bytes, region.kind});
    }
    const auto raw = TargetFrontendSnapshot::create(
        image_, kPeSizeOfImage, epoch_, inputs, output);
    return raw == TargetFrontendRawError::ok ? BuildError::ok
                                              : BuildError::snapshot_rejected;
  }

  std::uintptr_t image_{};
  std::uint64_t epoch_{};
  std::size_t claimed_bytes_{};
  std::vector<Claim> claims_;
};

}  // namespace

TargetFrontendSnapshotBuildError build_current_process_frontend_snapshot_v1(
    const std::uintptr_t primary_image,
    const std::uint64_t epoch,
    const TargetFrontendSnapshotRoots& roots,
    TargetFrontendSnapshot& snapshot) noexcept {
  try {
    if (primary_image == 0 || epoch == 0) return BuildError::invalid_argument;
#if !defined(GORE_AS_CAPTURE_TEST_TARGET)
    if (primary_image != reinterpret_cast<std::uintptr_t>(GetModuleHandleW(nullptr))) {
      return BuildError::wrong_primary_image;
    }
#endif
    Builder builder(primary_image, epoch);
    return builder.build(roots, snapshot);
  } catch (...) {
    return BuildError::limit_exceeded;
  }
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_frontend_snapshot_builder_selftest_v1() noexcept {
  // The broad raw-materializer fixture exercises every populated edge. This fixture proves that
  // the production reader accepts only current-process typed extents and fails before publishing
  // a snapshot on root-shape drift, ownership drift and invalid delegate state.
  try {
    constexpr std::size_t image_bytes = kPeSizeOfImage;
    auto* const image = static_cast<std::byte*>(
        VirtualAlloc(nullptr, image_bytes, MEM_RESERVE, PAGE_NOACCESS));
    if (image == nullptr) return false;
    const auto release = [&] { VirtualFree(image, 0, MEM_RELEASE); };
    const std::array image_rvas{
        kFNamePoolRva,
        frontend_target_layout::static_names_rva,
        frontend_target_layout::automatic_imports_rva,
        frontend_target_layout::use_editor_scripts_rva,
        frontend_target_layout::class_analyze_delegate_rva,
        frontend_target_layout::process_chunks_delegate_rva,
        frontend_target_layout::post_process_code_delegate_rva,
    };
    for (const auto rva : image_rvas) {
      const auto page = reinterpret_cast<void*>(
          (reinterpret_cast<std::uintptr_t>(image + rva)) & ~std::uintptr_t{0xfff});
      if (VirtualAlloc(page, 0x1000, MEM_COMMIT, PAGE_READWRITE) == nullptr) {
        release();
        return false;
      }
    }
    for (const auto rva : {frontend_target_layout::process_chunks_delegate_rva,
                           frontend_target_layout::post_process_code_delegate_rva}) {
      const std::int32_t threshold = 2;
      std::memcpy(image + rva + 0x10, &threshold, sizeof(threshold));
    }
    auto* const data = static_cast<std::byte*>(
        VirtualAlloc(nullptr, 0x4000, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE));
    if (data == nullptr) {
      release();
      return false;
    }
    const auto free_all = [&] {
      VirtualFree(data, 0, MEM_RELEASE);
      release();
    };
    const auto base = reinterpret_cast<std::uintptr_t>(data);
    const auto manager = base;
    const auto settings = base + 0x800;
    const auto preprocessor = base + 0x1000;
    std::memcpy(data + frontend_target_layout::manager_settings, &settings, sizeof(settings));
    const RawArray empty{};
    const RawArray empty_sparse{};
    const std::int32_t first_free = -1;
    const auto put_sparse_empty = [&](const std::uintptr_t address) {
      std::memcpy(reinterpret_cast<void*>(address), &empty_sparse, sizeof(empty_sparse));
      const std::uintptr_t no_secondary_allocation = 0;
      const std::int32_t no_bits = 0;
      std::memcpy(reinterpret_cast<void*>(address + kSparseSecondaryAllocation),
                  &no_secondary_allocation, sizeof(no_secondary_allocation));
      std::memcpy(reinterpret_cast<void*>(address + kSparseAllocationNum), &no_bits,
                  sizeof(no_bits));
      std::memcpy(reinterpret_cast<void*>(address + kSparseAllocationMax), &no_bits,
                  sizeof(no_bits));
      std::memcpy(reinterpret_cast<void*>(address + kSparseFirstFree), &first_free,
                  sizeof(first_free));
    };
    std::memcpy(
        reinterpret_cast<void*>(
            settings + frontend_target_layout::settings_preprocessor_flags),
        &empty, sizeof(empty));
    put_sparse_empty(manager + frontend_target_layout::manager_blueprint_specializations);
    put_sparse_empty(preprocessor);
    std::memcpy(reinterpret_cast<void*>(
                    preprocessor + frontend_target_layout::preprocessor_files),
                &empty, sizeof(empty));
    std::memcpy(image + frontend_target_layout::static_names_rva, &empty, sizeof(empty));
    TargetFrontendSnapshotRoots roots{};
    roots.phase = TargetFrontendSnapshotPhase::configuration;
    roots.manager = manager;
    roots.preprocessor = preprocessor;
    TargetFrontendSnapshot snapshot;
    const auto built = build_current_process_frontend_snapshot_v1(
        reinterpret_cast<std::uintptr_t>(image), 11, roots, snapshot);
    FrontendPreprocessorConfig config{};
    const bool ok = built == BuildError::ok && snapshot.epoch() == 11 &&
                    materialize_graph_hook_config_v1(snapshot, config) ==
                        TargetFrontendRawError::ok &&
                    !config.class_analyze_bound &&
                    !config.process_chunks_bound && !config.post_process_code_bound;
    roots.descriptor_array = base + 0x2000;
    TargetFrontendSnapshot rejected;
    const bool root_shape_rejected = build_current_process_frontend_snapshot_v1(
                                         reinterpret_cast<std::uintptr_t>(image), 12, roots,
                                         rejected) == BuildError::invalid_argument;
    roots.descriptor_array = 0;
    const std::uintptr_t image_settings = reinterpret_cast<std::uintptr_t>(image) + 0x1000;
    std::memcpy(data + frontend_target_layout::manager_settings, &image_settings,
                sizeof(image_settings));
    const bool ownership_rejected = build_current_process_frontend_snapshot_v1(
                                        reinterpret_cast<std::uintptr_t>(image), 13, roots,
                                        rejected) == BuildError::target_layout_drift;
    free_all();
    return ok && root_shape_rejected && ownership_rejected;
  } catch (...) {
    return false;
  }
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
