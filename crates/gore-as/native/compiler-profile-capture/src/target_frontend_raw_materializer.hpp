#pragma once

#include "target_frontend_observer.hpp"

#include <cstddef>
#include <cstdint>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace gore_as_capture::v1::instrumentation {

enum class TargetRawRegionKind : std::uint8_t {
  primary_image = 1,
  immutable_data = 2,
};

struct TargetRawRegionInput final {
  std::uintptr_t target_address{};
  const std::byte* bytes{};
  std::size_t byte_count{};
  TargetRawRegionKind kind{};
};

enum class TargetFrontendRawError : std::uint32_t {
  ok = 0,
  invalid_argument,
  invalid_snapshot,
  address_overflow,
  unreadable_address,
  wrong_region_kind,
  invalid_container,
  limit_exceeded,
  invalid_utf16,
  invalid_utf8,
  invalid_fname,
  invalid_shared_owner,
  duplicate_identity,
  cyclic_ownership,
  target_layout_drift,
  unresolved_semantics,
};

// An immutable, pointer-preserving copy of the exact regions needed by one observer phase.  A
// production coordinator must construct it while the target is quiescent and give each phase a
// fresh nonzero epoch.  The materializer never dereferences a target pointer directly and never
// retains an input buffer after create() returns.
class TargetFrontendSnapshot final {
 public:
  TargetFrontendSnapshot() = default;
  TargetFrontendSnapshot(TargetFrontendSnapshot&&) noexcept = default;
  TargetFrontendSnapshot& operator=(TargetFrontendSnapshot&&) noexcept = default;
  TargetFrontendSnapshot(const TargetFrontendSnapshot&) = delete;
  TargetFrontendSnapshot& operator=(const TargetFrontendSnapshot&) = delete;

  static TargetFrontendRawError create(
      std::uintptr_t primary_image,
      std::uint32_t primary_image_bytes,
      std::uint64_t epoch,
      std::span<const TargetRawRegionInput> regions,
      TargetFrontendSnapshot& output) noexcept;

  TargetFrontendRawError read(
      std::uintptr_t address,
      std::span<std::byte> output,
      TargetRawRegionKind required_kind) const noexcept;
  TargetFrontendRawError read_any(
      std::uintptr_t address,
      std::span<std::byte> output) const noexcept;
  bool is_image_address(std::uintptr_t address, std::size_t bytes = 1) const noexcept;
  bool is_data_address(std::uintptr_t address, std::size_t bytes = 1) const noexcept;
  std::uintptr_t primary_image() const noexcept { return primary_image_; }
  std::uint32_t primary_image_bytes() const noexcept { return primary_image_bytes_; }
  std::uint64_t epoch() const noexcept { return epoch_; }

 private:
  struct Region final {
    std::uintptr_t target_address{};
    TargetRawRegionKind kind{};
    std::vector<std::byte> bytes;
  };

  const Region* find_region(std::uintptr_t address, std::size_t bytes) const noexcept;

  std::uintptr_t primary_image_{};
  std::uint32_t primary_image_bytes_{};
  std::uint64_t epoch_{};
  std::vector<Region> regions_;
};

struct TargetRawFName final {
  std::uint32_t comparison_index{};
  std::uint32_t number{};
};
static_assert(sizeof(TargetRawFName) == 8);

struct TargetFrontendRawChunk final {
  std::uint8_t type{};
  std::string content;
  std::string comment;
  bool has_name_space{};
  std::string name_space;
  bool has_class_descriptor{};
  std::string class_name;
  std::int32_t file_line_number{};
  std::int32_t chunk_start{};
  std::int32_t chunk_end{};
};

struct TargetFrontendRawFile final {
  std::string module_name;
  std::string absolute_path;
  std::string relative_path;
  std::string raw_code;
  std::string processed_code;
  std::vector<std::string> generated_code;
  std::vector<TargetFrontendRawChunk> chunks;
};

struct TargetFrontendGraphHookBindings final {
  struct RawDelegateState final {
    std::uintptr_t invocation_list{};
    std::int32_t num{};
    std::int32_t capacity{};
    std::int32_t compaction_threshold{};
    std::int32_t broadcast_count{};
  };
  bool class_analyze_bound{};
  std::uint32_t class_analyze_active_bindings{};
  bool process_chunks_bound{};
  bool post_process_code_bound{};
  std::uint32_t diagnostic_delegate{};
  RawDelegateState class_analyze_state{};
  RawDelegateState process_chunks_state{};
  RawDelegateState post_process_code_state{};
};

struct TargetFrontendNativeSuperRaw final {
  bool present{};
  FrontendNativeClassWitness witness;
};

enum class TargetFrontendGraphSource : std::uint8_t {
  chunk_content = 1,
  processed_code = 2,
  module_descriptors = 3,
};

TargetFrontendRawError materialize_fname_v1(
    const TargetFrontendSnapshot& snapshot,
    TargetRawFName raw,
    std::string& spelling) noexcept;

// BuildID 24539464 has no graph-hook binding path. Both exact 24-byte image objects must retain
// their static empty state; a pointer/count/capacity, compaction or broadcast-lock drift rejects
// the phase instead of attempting to serialize an unsupported mutable callback.
TargetFrontendRawError materialize_graph_hook_bindings_v1(
    const TargetFrontendSnapshot& snapshot,
    TargetFrontendGraphHookBindings& bindings) noexcept;

TargetFrontendRawError materialize_graph_hook_config_v1(
    const TargetFrontendSnapshot& snapshot,
    FrontendPreprocessorConfig& config) noexcept;

TargetFrontendRawError materialize_preprocessor_flags_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t preprocessor,
    std::vector<FrontendFlag>& flags) noexcept;

TargetFrontendRawError materialize_settings_flags_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t manager,
    std::vector<FrontendFlag>& flags) noexcept;

TargetFrontendRawError materialize_blueprint_specializations_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t manager,
    std::vector<std::string>& specializations) noexcept;

TargetFrontendRawError materialize_static_fnames_v1(
    const TargetFrontendSnapshot& snapshot,
    std::vector<FrontendFNameComparison>& names) noexcept;

TargetFrontendRawError materialize_uclass_witness_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t uclass,
    std::string_view angelscript_type_name,
    std::uint64_t property_offset,
    FrontendNativeClassWitness& witness) noexcept;

// Reads the exact UClass::GetPropertiesSize field from the immutable snapshot instead of
// accepting a caller-supplied layout value.
TargetFrontendRawError materialize_native_class_witness_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t uclass,
    std::string_view angelscript_type_name,
    FrontendNativeClassWitness& witness) noexcept;

// Returns the exact path of UObject::ClassPrivate. This lets the production collector
// distinguish UClass user data from UScriptStruct and unrelated host user-data pointers.
TargetFrontendRawError materialize_uobject_class_path_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t object,
    std::string& class_path) noexcept;

// Derives both the Angelscript spelling and native property offset from the exact class
// descriptor/UClass pair. A script superclass yields present=false; callers never supply an
// offset or reinterpret CodeSuperClass as a semantic value.
TargetFrontendRawError materialize_class_native_super_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t class_descriptor_shared,
    TargetFrontendNativeSuperRaw& native_super) noexcept;

TargetFrontendRawError materialize_preprocessor_files_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t preprocessor,
    std::vector<TargetFrontendRawFile>& files) noexcept;

TargetFrontendRawError materialize_preprocessor_graph_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t preprocessor,
    TargetFrontendGraphSource source,
    std::vector<FrontendGraphModule>& modules) noexcept;

TargetFrontendRawError materialize_module_descriptor_graph_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t descriptor_array,
    std::vector<FrontendGraphModule>& modules) noexcept;

TargetFrontendRawError materialize_class_analyze_frame_v1(
    const TargetFrontendSnapshot& snapshot,
    std::uintptr_t file,
    std::uintptr_t generated_statics_fstring,
    std::uintptr_t class_descriptor_shared,
    std::uintptr_t has_statics,
    FrontendClassFrame& frame) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_frontend_raw_materializer_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
