#pragma once

#include "target_frontend_raw_materializer.hpp"

#include <cstdint>

namespace gore_as_capture::v1::instrumentation {

// Each phase admits exactly one statically witnessed root shape. Addresses are capabilities from
// the pinned observer frame; callers cannot supply arbitrary address/length pairs.
enum class TargetFrontendSnapshotPhase : std::uint32_t {
  configuration = 1,
  module_descriptors = 2,
  class_analyze = 3,
  native_class = 4,
  hook_bindings = 5,
  settings_configuration = 6,
};

struct TargetFrontendSnapshotRoots final {
  TargetFrontendSnapshotPhase phase{};
  std::uintptr_t manager{};
  std::uintptr_t preprocessor{};
  std::uintptr_t descriptor_array{};
  std::uintptr_t file{};
  std::uintptr_t generated_statics_fstring{};
  std::uintptr_t class_descriptor_shared{};
  std::uintptr_t has_statics{};
  std::uintptr_t uclass{};
};

enum class TargetFrontendSnapshotBuildError : std::uint32_t {
  ok = 0,
  invalid_argument,
  wrong_primary_image,
  address_overflow,
  unreadable_range,
  wrong_ownership_region,
  invalid_container,
  invalid_shared_owner,
  invalid_fname,
  cyclic_ownership,
  target_layout_drift,
  lifetime_drift,
  limit_exceeded,
  snapshot_rejected,
  configuration_settings_flags,
  configuration_blueprint_specializations,
  configuration_preprocessor,
  configuration_static_fnames,
  configuration_hook_bindings,
  file_module_name_container,
  file_absolute_path_container,
  file_relative_path_container,
  file_raw_code_container,
  file_processed_code_container,
  file_generated_array_container,
  file_generated_string_container,
  file_chunk_blocks_container,
  file_chunk_count_container,
  file_chunk_container,
  class_name_container,
  class_super_name_container,
  class_compose_container,
  class_namespace_container,
};

// Bounded CurrentProcess -> immutable snapshot boundary. It never opens a process, scans a page
// for possible pointers, or copies an entire PE/VirtualQuery region. Only the exact typed extents
// reachable from TargetFrontendSnapshotRoots are admitted.
TargetFrontendSnapshotBuildError build_current_process_frontend_snapshot_v1(
    std::uintptr_t primary_image,
    std::uint64_t epoch,
    const TargetFrontendSnapshotRoots& roots,
    TargetFrontendSnapshot& snapshot) noexcept;

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
bool target_frontend_snapshot_builder_selftest_v1() noexcept;
#endif

}  // namespace gore_as_capture::v1::instrumentation
