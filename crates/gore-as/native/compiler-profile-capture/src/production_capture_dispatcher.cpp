#include "production_capture_dispatcher.hpp"
#include "bridge_internal.hpp"
#include "live_bootstrap_internal.hpp"

#include "gore_as_capture/instrumentation.hpp"
#include "target_frontend_raw_materializer.hpp"
#include "target_frontend_snapshot_builder.hpp"
#include "target_registration_observer.hpp"
#include "target_snapshot.hpp"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <cstring>
#include <limits>
#include <memory>
#include <new>
#include <span>
#include <string>
#include <utility>
#include <vector>

namespace gore_as_capture::v1::instrumentation {
namespace {

namespace registration = gore_as_capture::v1::instrumentation::registration;

constexpr std::size_t kMaximumRegistrationDepth = 64;
constexpr std::size_t kSettingsBytes = 0x76;
constexpr std::size_t kPreprocessorBytes = 0x108;
// The selected generation's GetStaticJITInfo is exactly
//   48 8B 05 81 B3 49 05 C3  mov rax,[rip+0549B381h]; ret
// Its prolog is part of the fail-closed patch preflight. Development-data
// generation exits before calling this getter, so read its identical backing
// slot after frontend capture instead of requiring an unreachable invocation.
constexpr std::uint32_t kStaticJitInfoStorageRva = kStaticJitInfoGlobalRva;

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

bool read_current(const std::uintptr_t address, const std::span<std::byte> output) noexcept {
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

template <typename Type>
bool read_current_value(const std::uintptr_t address, Type& value) noexcept {
  static_assert(std::is_trivially_copyable_v<Type>);
  return read_current(address, {reinterpret_cast<std::byte*>(&value), sizeof(value)});
}

bool valid_utf8(const std::span<const char> bytes) noexcept {
  std::size_t cursor = 0;
  while (cursor < bytes.size()) {
    const auto first = static_cast<std::uint8_t>(bytes[cursor]);
    if (first <= 0x7f) {
      ++cursor;
      continue;
    }
    std::size_t trailing = 0;
    std::uint32_t codepoint = 0;
    std::uint32_t minimum = 0;
    if ((first & 0xe0u) == 0xc0u) {
      trailing = 1;
      codepoint = first & 0x1fu;
      minimum = 0x80;
    } else if ((first & 0xf0u) == 0xe0u) {
      trailing = 2;
      codepoint = first & 0x0fu;
      minimum = 0x800;
    } else if ((first & 0xf8u) == 0xf0u) {
      trailing = 3;
      codepoint = first & 0x07u;
      minimum = 0x10000;
    } else {
      return false;
    }
    if (trailing > bytes.size() - cursor - 1) return false;
    for (std::size_t index = 1; index <= trailing; ++index) {
      const auto next = static_cast<std::uint8_t>(bytes[cursor + index]);
      if ((next & 0xc0u) != 0x80u) return false;
      codepoint = (codepoint << 6) | (next & 0x3fu);
    }
    if (codepoint < minimum || codepoint > 0x10ffff ||
        (codepoint >= 0xd800 && codepoint <= 0xdfff)) {
      return false;
    }
    cursor += trailing + 1;
  }
  return true;
}

bool copy_bounded_utf8(
    const std::uintptr_t source,
    RawRegistrationArgument& projection) noexcept {
  constexpr std::size_t kMaximumTextBytes = 1024;
  if (source == 0) return false;
  for (std::size_t index = 0; index <= kMaximumTextBytes; ++index) {
    char value = 0;
    if (!read_current_value(source + index, value)) return false;
    if (value == '\0') {
      if (!valid_utf8({projection.text.data(), index})) return false;
      projection.text_bytes = static_cast<std::uint32_t>(index);
      projection.text[index] = '\0';
      return true;
    }
    if (index == kMaximumTextBytes) return false;
    projection.text[index] = value;
  }
  return false;
}

bool registration_argument(
    const ProductionMachineFrame& frame,
    const std::uint8_t source,
    std::uintptr_t& value) noexcept {
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_RDX_V1:
      value = frame.rdx;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R8_V1:
      value = frame.r8;
      return true;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_R9_V1:
      value = frame.r9;
      return true;
    default:
      break;
  }
  std::uint32_t offset = 0;
  switch (source) {
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_28_V1: offset = 0x28; break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_30_V1: offset = 0x30; break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_38_V1: offset = 0x38; break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_40_V1: offset = 0x40; break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_48_V1: offset = 0x48; break;
    case GORE_AS_CAPTURE_ARGUMENT_SOURCE_STACK_50_V1: offset = 0x50; break;
    default: return false;
  }
  return frame.rsp <= std::numeric_limits<std::uintptr_t>::max() - offset &&
         read_current_value(frame.rsp + offset, value);
}

bool extract_registration(
    const std::size_t hook_index,
    const ProductionMachineFrame& frame,
    RawRegistrationEntry& result,
    std::uint32_t& failure_detail) noexcept {
  if (hook_index >= registration::kPinnedRegistrationHooks.size() || frame.rcx == 0) {
    failure_detail = 1;
    return false;
  }
  const auto& hook = registration::kPinnedRegistrationHooks[hook_index];
  RawRegistrationEntry projected{};
  projected.kind = hook.kind;
  projected.engine_capability = frame.rcx;
  projected.argument_count = hook.argument_count;
  for (std::size_t index = 0; index < hook.argument_count; ++index) {
    const auto& contract = hook.arguments[index];
    auto& argument = projected.arguments[index];
    argument.semantic = contract.semantic;
    std::uintptr_t value = 0;
    if (!registration_argument(frame, contract.source, value)) {
      failure_detail = 0x100u + static_cast<std::uint32_t>(index);
      return false;
    }
    switch (contract.semantic) {
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_UTF8_V1:
        if (!copy_bounded_utf8(value, argument)) {
          failure_detail = 0x200u + static_cast<std::uint32_t>(index);
          return false;
        }
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_SFUNC_PTR_REF_V1: {
        constexpr auto bytes = layout_v23300::donor::function_pointer_descriptor_bytes;
        if (!read_current(value, {argument.opaque_descriptor.data(), bytes})) {
          failure_detail = 0x300u + static_cast<std::uint32_t>(index);
          return false;
        }
        const auto flag = std::to_integer<std::uint8_t>(
            argument.opaque_descriptor[layout_v23300::donor::function_pointer_descriptor_flag]);
        if (flag == 0 || flag > 3) {
          failure_detail = 0x400u + static_cast<std::uint32_t>(index);
          return false;
        }
        argument.opaque_descriptor_bytes = static_cast<std::uint32_t>(bytes);
        argument.pointer_capability = value;
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALLER_VALUE_REF_V1: {
        constexpr auto bytes = layout_v23300::donor::function_caller_descriptor_bytes;
        if (!read_current(value, {argument.opaque_descriptor.data(), bytes})) {
          failure_detail = 0x500u + static_cast<std::uint32_t>(index);
          return false;
        }
        std::int32_t type = 0;
        std::uintptr_t callable = 0;
        std::memcpy(&callable, argument.opaque_descriptor.data(), sizeof(callable));
        std::memcpy(&type,
                    argument.opaque_descriptor.data() +
                        layout_v23300::donor::function_caller_descriptor_type,
                    sizeof(type));
        if (type < 0 || type > 2 || (type == 0) != (callable == 0)) {
          failure_detail = 0x600u + static_cast<std::uint32_t>(index);
          return false;
        }
        argument.scalar = static_cast<std::uint32_t>(type);
        argument.pointer_capability = callable;
        argument.opaque_descriptor_bytes = static_cast<std::uint32_t>(bytes);
        break;
      }
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_CALL_CONVENTION_U32_V1:
        if (static_cast<std::uint32_t>(value) > 8) {
          failure_detail = 0x700u + static_cast<std::uint32_t>(index);
          return false;
        }
        argument.scalar = static_cast<std::uint32_t>(value);
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BOOL_V1:
        if (static_cast<std::uint8_t>(value) > 1) {
          failure_detail = 0x800u + static_cast<std::uint32_t>(index);
          return false;
        }
        argument.scalar = static_cast<std::uint8_t>(value);
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_POINTER_TOKEN_V1:
        argument.pointer_capability = value;
        break;
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_I32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_BEHAVIOUR_I32_V1:
      case GORE_AS_CAPTURE_ARGUMENT_SEMANTIC_U32_V1:
        argument.scalar = static_cast<std::uint32_t>(value);
        break;
      default:
        failure_detail = 0x900u + static_cast<std::uint32_t>(index);
        return false;
    }
  }
  result = projected;
  return true;
}

struct BindMetadata final {
  std::int32_t order{};
  std::uintptr_t callback{};
  std::uintptr_t engine{};
  bool final_callback{};
};

bool extract_bind(
    const std::uintptr_t image,
    const ProductionMachineFrame& frame,
    BindMetadata& output) noexcept {
  if (frame.rbx == 0 || frame.r15 == 0 || frame.r12 == 0 || frame.rdi <= image ||
      frame.rdi - image >= kPeSizeOfImage ||
      frame.r15 > std::numeric_limits<std::uintptr_t>::max() - kBindRecordStride ||
      frame.r15 + kBindRecordStride > frame.r12) {
    return false;
  }
  BindMetadata value{};
  value.callback = frame.rdi;
  value.final_callback = frame.r15 + kBindRecordStride == frame.r12;
  if (!read_current_value(frame.rbx + kManagerEngineOffset, value.engine) ||
      !read_current_value(frame.r15 + kBindOrderOffset, value.order) || value.engine == 0) {
    return false;
  }
  output = value;
  return true;
}

bool same_bind(const BindMetadata& left, const BindMetadata& right) noexcept {
  return left.order == right.order && left.callback == right.callback &&
         left.engine == right.engine && left.final_callback == right.final_callback;
}

bool extract_build_jit(
    const std::uintptr_t manager,
    const std::uint32_t build_identifier,
    const std::uintptr_t jit,
    gore_as_capture_build_jit_v1& result) noexcept {
  if (manager == 0 || build_identifier != kBuildIdentifier ||
      !live_capture_target_inputs_verified_v1()) {
    return false;
  }
  std::uintptr_t precompiled = 0;
  if (!read_current_value(manager + kManagerPrecompiledDataOffset, precompiled) ||
      precompiled == 0) {
    return false;
  }
  result = {};
  result.struct_size = sizeof(result);
  result.build_identifier = build_identifier;
  std::memcpy(result.precompiled_guid, kPrecompiledGuid.data(), kPrecompiledGuid.size());
  result.shipping_cache_matches = 1;
  if (jit != 0) {
    if (!read_current(jit, {reinterpret_cast<std::byte*>(result.compiled_jit_guid),
                            sizeof(result.compiled_jit_guid)})) {
      return false;
    }
    result.jit_info_present = 1;
    result.jit_guid_matches =
        std::memcmp(result.precompiled_guid, result.compiled_jit_guid,
                    sizeof(result.precompiled_guid)) == 0;
    result.jit_database_cleared = result.jit_guid_matches == 0;
  }
  result.as_reference_debugging = kAsReferenceDebugging;
  result.fork_opcode_table_201_212_present = kForkOpcodeTable201Through212Present;
  result.reference_debug_opcodes_emittable = kReferenceDebugOpcodesEmittable;
  result.resolve_object_ptr_callback_registered = kResolveObjectPtrCallbackRegistered;
  result.get_build_identifier_rva = kRvaGetBuildIdentifier;
  result.get_static_jit_info_rva = kRvaGetStaticJitInfo;
  return true;
}

bool target_flag_set(std::vector<FrontendFlag>& flags) {
  constexpr std::array required{
      std::pair{"COOK_COMMANDLET", false}, std::pair{"EDITOR", false},
      std::pair{"EDITORONLY_DATA", false}, std::pair{"RELEASE", true},
      std::pair{"TEST", false}, std::pair{"WITH_SERVER_CODE", true}};
  for (const auto& required_flag : required) {
    const auto found = std::find_if(flags.begin(), flags.end(), [&](const auto& value) {
      return value.name == required_flag.first;
    });
    if (found != flags.end()) {
      if (found->value != required_flag.second) return false;
    } else {
      flags.push_back({required_flag.first, required_flag.second});
    }
  }
  std::sort(flags.begin(), flags.end(),
            [](const auto& left, const auto& right) { return left.name < right.name; });
  return std::adjacent_find(flags.begin(), flags.end(), [](const auto& left, const auto& right) {
           return left.name == right.name;
         }) == flags.end();
}

}  // namespace

struct ProductionCaptureCoordinator::Impl final {
  struct PendingRegistration final {
    std::uint32_t site{};
    PendingRegistrationProjection projection;
  };

  bool fail() noexcept {
    failed = true;
    if (machine.needs_abort()) (void)machine.abort();
    return false;
  }

  bool owner() const noexcept { return owner_thread == GetCurrentThreadId(); }

  bool snapshot(
      const TargetFrontendSnapshotRoots& roots,
      TargetFrontendSnapshot& output) noexcept {
    if (++epoch == 0) return false;
    const auto status =
        build_current_process_frontend_snapshot_v1(image, epoch, roots, output);
    if (status != TargetFrontendSnapshotBuildError::ok) {
      if (status ==
              TargetFrontendSnapshotBuildError::configuration_settings_flags &&
          roots.manager != 0) {
        std::uintptr_t settings = 0;
        std::array<std::uint64_t, 5> sparse{};
        if (read_current_value(
                roots.manager + frontend_target_layout::manager_settings, settings) &&
            read_current(
                settings + frontend_target_layout::settings_preprocessor_flags,
                {reinterpret_cast<std::byte*>(sparse.data()),
                 sparse.size() * sizeof(sparse.front())})) {
          live_capture_note_registration_arguments_v1(
              "settings_flags", 14, nullptr, 0, sparse[0], sparse[1], sparse[2]);
        }
      } else if (status == TargetFrontendSnapshotBuildError::
                               configuration_blueprint_specializations &&
                 roots.manager != 0) {
        std::array<std::uint64_t, 8> header{};
        if (read_current(
                roots.manager +
                    frontend_target_layout::manager_blueprint_specializations,
                {reinterpret_cast<std::byte*>(header.data()),
                 header.size() * sizeof(header.front())})) {
          live_capture_note_container_header_v1(header);
          live_capture_note_registration_arguments_v1(
              "blueprint_set", 13, nullptr, 0, header[0], header[1], header[2]);
        }
      } else {
        live_capture_note_registration_arguments_v1(
            "snapshot_roots", 14, nullptr, 0,
            static_cast<std::uint32_t>(roots.phase), roots.manager,
            roots.preprocessor);
      }
      live_capture_note_failure_detail_v1(
          0xC000u + static_cast<std::uint32_t>(status));
      return false;
    }
    return true;
  }

  static bool resolve_pointer(
      void* const context,
      const std::uintptr_t pointer,
      std::uint32_t& token) noexcept {
    auto* const self = static_cast<Impl*>(context);
    return self != nullptr &&
           self->machine.intern_primary_image_pointer(pointer, token) ==
               ProductionCapturePhaseError::ok;
  }

  static bool intern_opaque_pointer(
      std::vector<std::uintptr_t>& pointers,
      const std::uintptr_t pointer,
      std::uint32_t& token) noexcept {
    if (pointer == 0) return false;
    const auto found = std::find(pointers.begin(), pointers.end(), pointer);
    if (found != pointers.end()) {
      token = static_cast<std::uint32_t>(std::distance(pointers.begin(), found));
      return true;
    }
    if (pointers.size() >= std::numeric_limits<std::uint32_t>::max()) return false;
    try {
      token = static_cast<std::uint32_t>(pointers.size());
      pointers.push_back(pointer);
      return true;
    } catch (...) {
      return false;
    }
  }

  static bool resolve_object_pointer(
      void* const context,
      const std::uintptr_t pointer,
      std::uint32_t& token) noexcept {
    auto* const self = static_cast<Impl*>(context);
    return self != nullptr &&
           intern_opaque_pointer(self->object_pointers, pointer, token);
  }

  static bool resolve_storage_pointer(
      void* const context,
      const std::uintptr_t pointer,
      std::uint32_t& token) noexcept {
    auto* const self = static_cast<Impl*>(context);
    return self != nullptr &&
           intern_opaque_pointer(self->storage_pointers, pointer, token);
  }

  bool try_build_jit() noexcept {
    if (build_emitted || !registry_complete || !have_build_id || !have_jit || manager == 0) {
      return true;
    }
    gore_as_capture_build_jit_v1 fact{};
    if (!extract_build_jit(manager, build_identifier, jit_pointer, fact)) {
      live_capture_note_registration_arguments_v1(
          "build_extract", 13, nullptr, 0, build_identifier, jit_pointer, manager);
      live_capture_note_failure_detail_v1(0xB790u);
      return false;
    }
    const auto build_status = machine.set_build_jit(fact);
    if (build_status != ProductionCapturePhaseError::ok) {
      std::uint64_t precompiled_guid0 = 0;
      std::uint64_t precompiled_guid1 = 0;
      std::memcpy(&precompiled_guid0, fact.precompiled_guid,
                  sizeof(precompiled_guid0));
      std::memcpy(&precompiled_guid1,
                  fact.precompiled_guid + sizeof(precompiled_guid0),
                  sizeof(precompiled_guid1));
      live_capture_note_registration_arguments_v1(
          "build_guid", 10, nullptr, 0, precompiled_guid0,
          precompiled_guid1,
          static_cast<std::uint64_t>(fact.shipping_cache_matches) |
              (static_cast<std::uint64_t>(fact.jit_info_present) << 8) |
              (static_cast<std::uint64_t>(fact.jit_guid_matches) << 16) |
              (static_cast<std::uint64_t>(fact.jit_database_cleared) << 24));
      live_capture_note_failure_detail_v1(
          0xB7A0u + static_cast<std::uint32_t>(build_status));
      return false;
    }
    build_emitted = true;
    if (!frontend_emitted) return true;
    const auto complete_status = machine.complete();
    if (complete_status != ProductionCapturePhaseError::ok) {
      live_capture_note_failure_detail_v1(
          0xB7B0u + static_cast<std::uint32_t>(complete_status));
      return false;
    }
    return true;
  }

  bool begin_bind(const ProductionMachineFrame& frame) noexcept {
    if (registry_complete) return false;
    if (active_bind) {
      if (!synthetic_bind || !registrations || !pending.empty() || engine == 0 ||
          machine.end_bind(registry_snapshot) != ProductionCapturePhaseError::ok) {
        return false;
      }
      active_bind = false;
      synthetic_bind = false;
      ++bind_ordinal;
    }
    BindMetadata metadata{};
    PublicRegistrySnapshot baseline{};
    if (!extract_bind(image, frame, metadata) ||
        (engine != 0 && engine != metadata.engine)) {
      return false;
    }
    if (registrations) {
      baseline = registry_snapshot;
    } else if (capture_public_registry_snapshot_v23300(
                   image, kPeSizeOfImage, metadata.engine, baseline) != SnapshotError::ok) {
      return false;
    }
    engine = metadata.engine;
    if (manager != 0 && manager != frame.rbx) return false;
    manager = frame.rbx;
    if (!registrations) {
      registrations = std::make_unique<TargetRegistrationObserver>(
          image, kPeSizeOfImage, engine,
          PointerTokenResolver{
              this, resolve_pointer, resolve_object_pointer, resolve_storage_pointer});
      if (!registrations ||
          registrations->begin_observation(baseline.counts) != RegistrationObserverError::ok) {
        return false;
      }
      registry_snapshot = baseline;
    }
    std::uint32_t token = 0;
    if (machine.intern_primary_image_pointer(metadata.callback, token) !=
            ProductionCapturePhaseError::ok ||
        machine.begin_bind(metadata.order, token, baseline) !=
            ProductionCapturePhaseError::ok) {
      return false;
    }
    bind = metadata;
    active_bind = true;
    return true;
  }

  bool end_bind(const ProductionMachineFrame& frame) noexcept {
    BindMetadata metadata{};
    PublicRegistrySnapshot final_snapshot{};
    if (!active_bind || synthetic_bind || !registrations || !pending.empty() ||
        !extract_bind(image, frame, metadata) || !same_bind(bind, metadata)) {
      return false;
    }
    if (metadata.final_callback) {
      if (capture_public_registry_snapshot_v23300(
              image, kPeSizeOfImage, engine, final_snapshot) != SnapshotError::ok) {
        return false;
      }
      RegistryCounts projected_counts{};
      if (registrations->projected_counts(projected_counts) !=
          RegistrationObserverError::ok) {
        return false;
      }
      live_capture_note_registry_counts_v1(projected_counts, final_snapshot.counts);
      registry_snapshot = final_snapshot;
    } else {
      final_snapshot = registry_snapshot;
    }
    if (machine.end_bind(final_snapshot) != ProductionCapturePhaseError::ok) return false;
    active_bind = false;
    if (!metadata.final_callback) {
      ++bind_ordinal;
      return true;
    }
    std::string support;
    std::vector<std::string> final_state;
    std::vector<std::vector<std::string>> replacement_deltas;
    const auto finalize_status = registrations->finalize_registry(
        bind_ordinal + 1, replacement_deltas, support);
    if (finalize_status != RegistrationObserverError::ok) {
      live_capture_note_failure_detail_v1(
          0x9700u + static_cast<std::uint32_t>(finalize_status));
      return false;
    }
    if (machine.replace_registry_deltas(std::move(replacement_deltas)) !=
        ProductionCapturePhaseError::ok) {
      live_capture_note_failure_detail_v1(0x9801u);
      return false;
    }
    const auto final_state_status = registrations->enumerate_post_bind_final_state(
        final_snapshot.counts, final_state);
    if (final_state_status != RegistrationObserverError::ok) {
      live_capture_note_failure_detail_v1(
          0x9900u + static_cast<std::uint32_t>(final_state_status));
      return false;
    }
    if (machine.complete_registry(std::move(support), std::move(final_state)) !=
        ProductionCapturePhaseError::ok) {
      live_capture_note_failure_detail_v1(0x9A01u);
      return false;
    }
    registry_complete = true;
    return try_build_jit();
  }

  bool registration_before(
      const std::uint32_t site,
      const ProductionMachineFrame& frame) {
    live_capture_note_observer_stage_v1(0x100u);
    RawRegistrationEntry raw{};
    PendingRegistration entry{};
    entry.site = site;
    std::uint32_t extraction_detail = 0;
    if (!extract_registration(
            site - kProductionBaseSiteCount, frame, raw, extraction_detail)) {
      live_capture_note_failure_detail_v1(0x9200u + extraction_detail);
      return false;
    }
    live_capture_note_observer_stage_v1(0x101u);
    live_capture_note_registration_arguments_v1(
        raw.arguments[0].text.data(), raw.arguments[0].text_bytes,
        raw.argument_count > 1 ? raw.arguments[1].text.data() : nullptr,
        raw.argument_count > 1 ? raw.arguments[1].text_bytes : 0,
        raw.arguments[0].scalar,
        raw.argument_count > 1 ? raw.arguments[1].scalar : 0,
        raw.argument_count > 2 ? raw.arguments[2].scalar : 0);
    if (engine != 0 && raw.engine_capability != engine) {
      live_capture_note_failure_detail_v1(0x9003u);
      return false;
    }
    if (!active_bind) {
      PublicRegistrySnapshot baseline{};
      if (registry_complete || raw.engine_capability == 0) {
        live_capture_note_failure_detail_v1(0x9004u);
        return false;
      }
      const bool first_bootstrap_registration =
          !registrations && bind_ordinal == 0 && site == kProductionBaseSiteCount;
      const auto snapshot_status =
          registrations
              ? (baseline = registry_snapshot, SnapshotError::ok)
              : (first_bootstrap_registration
                     ? empty_public_registry_snapshot_v23300(baseline)
                     : capture_public_registry_snapshot_v23300(
                           image, kPeSizeOfImage, raw.engine_capability, baseline));
      if (snapshot_status != SnapshotError::ok) {
        live_capture_note_failure_detail_v1(
            0x9600u + static_cast<std::uint32_t>(snapshot_status));
        return false;
      }
      if (!registrations && !first_bootstrap_registration) {
        live_capture_note_failure_detail_v1(0x9008u);
        return false;
      }
      engine = raw.engine_capability;
      if (!registrations) {
        registrations = std::make_unique<TargetRegistrationObserver>(
            image, kPeSizeOfImage, engine,
            PointerTokenResolver{
                this, resolve_pointer, resolve_object_pointer, resolve_storage_pointer});
        if (!registrations ||
            registrations->begin_observation(baseline.counts) !=
                RegistrationObserverError::ok) {
          live_capture_note_failure_detail_v1(0x9005u);
          return false;
        }
        registry_snapshot = baseline;
      }
      const auto registration_index = site - kProductionBaseSiteCount;
      if (registration_index >= registration::kPinnedRegistrationHooks.size()) {
        live_capture_note_failure_detail_v1(0x9006u);
        return false;
      }
      std::uint32_t token = 0;
      const auto callback =
          image + registration::kPinnedRegistrationHooks[registration_index].function_rva;
      if (machine.intern_primary_image_pointer(callback, token) !=
              ProductionCapturePhaseError::ok ||
          machine.begin_bind(
              std::numeric_limits<std::int32_t>::min() +
                  static_cast<std::int32_t>(bind_ordinal),
              token, baseline) != ProductionCapturePhaseError::ok) {
        live_capture_note_failure_detail_v1(0x9007u);
        return false;
      }
      active_bind = true;
      synthetic_bind = true;
    }
    if (!registrations || pending.size() == kMaximumRegistrationDepth) {
      live_capture_note_failure_detail_v1(0x9001u);
      return false;
    }
    live_capture_note_observer_stage_v1(0x102u);
    const auto prepared = registrations->prepare(raw, entry.projection);
    if (prepared != RegistrationObserverError::ok) {
      live_capture_note_failure_detail_v1(
          0x9100u + static_cast<std::uint32_t>(prepared));
      return false;
    }
    live_capture_note_observer_stage_v1(0x103u);
    pending.push_back(std::move(entry));
    live_capture_note_observer_stage_v1(0);
    return true;
  }

  bool registration_after(
      const std::uint32_t site,
      const ProductionMachineFrame& frame) {
    live_capture_note_observer_stage_v1(0x200u);
    live_capture_note_registration_result_v1(
        site, static_cast<std::int32_t>(frame.rax));
    if (!active_bind) return false;
    if (!registrations) {
      live_capture_note_failure_detail_v1(0x9301u);
      return false;
    }
    if (pending.empty()) {
      live_capture_note_failure_detail_v1(0x9302u);
      return false;
    }
    if (pending.back().site != site) {
      live_capture_note_failure_detail_v1(
          0x9300u + (pending.back().site & 0xffu));
      return false;
    }
    auto entry = std::move(pending.back());
    pending.pop_back();
    // AngelScript's registration API uses asALREADY_REGISTERED (-13) for a
    // deliberate no-op.  The game's bind layer relies on this for references
    // to types installed by earlier bootstrap/add-on registration.  It does
    // not advance registry state and therefore must not become a replay entry.
    if (static_cast<std::int32_t>(frame.rax) == -13) {
      live_capture_note_observer_stage_v1(0);
      return true;
    }
    if (registrations->is_core_intrinsic(
            entry.projection, static_cast<std::int32_t>(frame.rax))) {
      live_capture_note_observer_stage_v1(0);
      return true;
    }
    CompletedRegistrationProjection completed{};
    const auto completed_status = registrations->complete(
        bind_ordinal, entry.projection, static_cast<std::int32_t>(frame.rax),
        completed);
    if (completed_status != RegistrationObserverError::ok) {
      live_capture_note_failure_detail_v1(
          completed_status == RegistrationObserverError::result_rejected
              ? static_cast<std::uint32_t>(frame.rax)
              : 0x9400u + static_cast<std::uint32_t>(completed_status));
      return false;
    }
    RegistryCounts projected_counts{};
    PublicRegistrySnapshot projected_snapshot{};
    if (registrations->projected_counts(projected_counts) !=
            RegistrationObserverError::ok ||
        advance_public_registry_witness_v1(
            registry_snapshot, projected_counts, completed.delta_json,
            projected_snapshot) != SnapshotError::ok) {
      live_capture_note_failure_detail_v1(0x9400u +
                                          static_cast<std::uint32_t>(
                                              RegistrationObserverError::registry_count_drift));
      return false;
    }
    live_capture_note_observer_stage_v1(0x500u);
    const auto delta_status =
        machine.add_registry_delta(std::move(completed.delta_json));
    live_capture_note_observer_stage_v1(0x501u);
    if (delta_status != ProductionCapturePhaseError::ok) {
      live_capture_note_failure_detail_v1(
          0x9500u + static_cast<std::uint32_t>(delta_status));
      return false;
    }
    registry_snapshot = projected_snapshot;
    live_capture_note_observer_stage_v1(0);
    return true;
  }

  bool begin_frontend(const ProductionMachineFrame& frame) noexcept {
    if (!registry_complete) {
      live_capture_note_failure_detail_v1(0xB001u);
      return false;
    }
    if (frontend_started || frame.rcx == 0) {
      live_capture_note_failure_detail_v1(0xB002u);
      return false;
    }
    std::uintptr_t target_engine = 0;
    if (!read_current_value(frame.rcx + kManagerEngineOffset, target_engine) ||
        target_engine != engine) {
      live_capture_note_failure_detail_v1(0xB003u);
      return false;
    }
    manager = frame.rcx;
    // Generation mutates FPrecompiledData's GUID before InitialCompile returns.
    // Capture the shipping-cache/JIT fact at the entry boundary, while the
    // manager still owns the exact loaded shipping state. The two getters are
    // pinned by the executable preflight; this path reads their known constant
    // result and backing slot without recursively invoking patched entries.
    if (!have_build_id) {
      build_identifier = kBuildIdentifier;
      have_build_id = true;
    }
    if (!have_jit) {
      if (!read_current_value(image + kStaticJitInfoStorageRva, jit_pointer)) {
        live_capture_note_failure_detail_v1(0xB004u);
        return false;
      }
      have_jit = true;
    }
    if (!try_build_jit()) {
      live_capture_note_failure_detail_v1(0xB005u);
      return false;
    }
    TargetFrontendSnapshot hook_snapshot;
    TargetFrontendSnapshotRoots roots{};
    roots.phase = TargetFrontendSnapshotPhase::hook_bindings;
    TargetFrontendGraphHookBindings bindings{};
    if (!snapshot(roots, hook_snapshot)) {
      live_capture_note_failure_detail_v1(0xB410u);
      return false;
    }
    const auto binding_status =
        materialize_graph_hook_bindings_v1(hook_snapshot, bindings);
    if (binding_status != TargetFrontendRawError::ok) {
      const auto* raw = &bindings.class_analyze_state;
      const char* label = "class_analyze";
      if (bindings.diagnostic_delegate == 2) {
        raw = &bindings.process_chunks_state;
        label = "process_chunks";
      } else if (bindings.diagnostic_delegate == 3) {
        raw = &bindings.post_process_code_state;
        label = "post_process_code";
      }
      live_capture_note_registration_arguments_v1(
          label, static_cast<std::uint32_t>(std::strlen(label)), nullptr, 0,
          raw->invocation_list,
          static_cast<std::uint64_t>(static_cast<std::uint32_t>(raw->num)) |
              (static_cast<std::uint64_t>(
                   static_cast<std::uint32_t>(raw->capacity))
               << 32),
          static_cast<std::uint64_t>(
              static_cast<std::uint32_t>(raw->compaction_threshold)) |
              (static_cast<std::uint64_t>(
                   static_cast<std::uint32_t>(raw->broadcast_count))
               << 32));
      live_capture_note_failure_detail_v1(
          0xB420u + static_cast<std::uint32_t>(binding_status));
      return false;
    }
    if (bindings.process_chunks_bound || bindings.post_process_code_bound) {
      live_capture_note_failure_detail_v1(
          0xB430u + (bindings.class_analyze_bound ? 1u : 0u) +
          (bindings.process_chunks_bound ? 2u : 0u) +
          (bindings.post_process_code_bound ? 4u : 0u));
      return false;
    }
    const auto observer_status =
        frontend.set_hook_bindings(bindings.class_analyze_bound, false, false);
    if (observer_status != FrontendObserverError::ok) {
      live_capture_note_failure_detail_v1(
          0xB440u + static_cast<std::uint32_t>(observer_status));
      return false;
    }
    hook_bindings = bindings;
    frontend_started = true;
    return true;
  }

  bool descriptor_after(const ProductionMachineFrame& frame) {
    if (!frontend_started || middle != FrontendBoundaryKind{} || saved_boundary_rcx == 0 ||
        saved_boundary_rdx == 0 || frame.rax != saved_boundary_rdx) {
      return false;
    }
    TargetFrontendSnapshot snapshot_value;
    TargetFrontendSnapshotRoots roots{};
    roots.phase = TargetFrontendSnapshotPhase::module_descriptors;
    roots.descriptor_array = saved_boundary_rdx;
    if (!snapshot(roots, snapshot_value) ||
        materialize_module_descriptor_graph_v1(
            snapshot_value, saved_boundary_rdx, descriptor_modules) !=
            TargetFrontendRawError::ok ||
        descriptor_modules.empty()) {
      return false;
    }
    middle = FrontendBoundaryKind::precompiled_descriptors_requested;
    return true;
  }

  bool preprocessor_after(const ProductionMachineFrame&) noexcept {
    if (!frontend_started || middle != FrontendBoundaryKind{} || saved_boundary_rcx == 0) {
      return false;
    }
    preprocessor = saved_boundary_rcx;
    middle = FrontendBoundaryKind::preprocessor_constructed;
    return true;
  }

  bool graph(
      const std::uint32_t site,
      const ProductionShimPhase phase,
      const ProductionMachineFrame& frame) {
    const bool before = phase == ProductionShimPhase::before;
    if (middle != FrontendBoundaryKind::preprocessor_constructed) {
      live_capture_note_failure_detail_v1(0xB501u);
      return false;
    }
    if (frame.rdx != preprocessor) {
      live_capture_note_registration_arguments_v1(
          "graph_frame", 11, nullptr, 0, frame.rcx, frame.rdx, preprocessor);
      live_capture_note_failure_detail_v1(0xB502u);
      return false;
    }
    if ((site == 23 &&
         ((before &&
           (process_pending || process_complete || post_pending || post_complete)) ||
          (!before &&
           (!process_pending || process_complete || post_pending || post_complete)))) ||
        (site == 24 &&
         ((before &&
           (!process_complete || process_pending || post_pending || post_complete)) ||
          (!before &&
           (!process_complete || process_pending || !post_pending || post_complete))))) {
      live_capture_note_failure_detail_v1(0xB503u);
      return false;
    }
    TargetFrontendSnapshot snapshot_value;
    TargetFrontendSnapshotRoots roots{};
    roots.phase = TargetFrontendSnapshotPhase::configuration;
    roots.manager = manager;
    roots.preprocessor = preprocessor;
    std::vector<FrontendGraphModule> modules;
    TargetFrontendGraphHookBindings bindings{};
    const auto source = site == 23 ? TargetFrontendGraphSource::chunk_content
                                   : TargetFrontendGraphSource::processed_code;
    if (!snapshot(roots, snapshot_value)) {
      live_capture_note_failure_detail_v1(0xB510u);
      return false;
    }
    const auto binding_status =
        materialize_graph_hook_bindings_v1(snapshot_value, bindings);
    if (binding_status != TargetFrontendRawError::ok) {
      live_capture_note_failure_detail_v1(
          0xB520u + static_cast<std::uint32_t>(binding_status));
      return false;
    }
    if (bindings.class_analyze_bound != hook_bindings.class_analyze_bound ||
        bindings.class_analyze_active_bindings !=
            hook_bindings.class_analyze_active_bindings ||
        bindings.process_chunks_bound || bindings.post_process_code_bound) {
      live_capture_note_failure_detail_v1(0xB530u);
      return false;
    }
    const auto graph_status = materialize_preprocessor_graph_v1(
        snapshot_value, preprocessor, source, modules);
    if (graph_status != TargetFrontendRawError::ok || modules.empty()) {
      live_capture_note_failure_detail_v1(
          0xB540u + static_cast<std::uint32_t>(graph_status));
      return false;
    }
    if (site == 23) {
      const auto status = before ? frontend.begin_process_chunks(modules)
                                 : frontend.complete_process_chunks(modules);
      if (status != FrontendObserverError::ok) return false;
      process_pending = before;
      process_complete = !before;
      return true;
    }
    const auto status = before ? frontend.begin_post_process_code(modules)
                               : frontend.complete_post_process_code(modules);
    if (status != FrontendObserverError::ok) return false;
    post_pending = before;
    post_complete = !before;
    if (!before) {
      final_frontend_snapshot = std::move(snapshot_value);
      final_frontend_snapshot_ready = true;
    }
    return true;
  }

  bool class_analyze(
      const ProductionShimPhase phase,
      const ProductionMachineFrame& frame) {
    const bool before = phase == ProductionShimPhase::before;
    if (middle != FrontendBoundaryKind::preprocessor_constructed ||
        !hook_bindings.class_analyze_bound ||
        (before ? (class_pending || frame.rbp == 0) : !class_pending)) {
      live_capture_note_failure_detail_v1(0xB600u);
      return false;
    }
    TargetFrontendSnapshotRoots roots{};
    if (before) {
      std::uintptr_t file = 0;
      if (!read_current_value(frame.rbp + 0x88, file) || file == 0) {
        live_capture_note_failure_detail_v1(0xB601u);
        return false;
      }
      roots.phase = TargetFrontendSnapshotPhase::class_analyze;
      roots.file = file;
      roots.generated_statics_fstring = frame.rdx;
      roots.class_descriptor_shared = frame.r8;
      roots.has_statics = frame.r9;
    } else {
      // The delegate call may clobber every volatile argument register.  The
      // pointed-to stack objects remain alive until this synchronous broadcast
      // returns, so reuse only the typed capabilities admitted before the call.
      roots = class_pending_roots;
    }
    TargetFrontendSnapshot snapshot_value;
    FrontendClassFrame projected{};
    if (!snapshot(roots, snapshot_value)) {
      return false;
    }
    const auto frame_status = materialize_class_analyze_frame_v1(
        snapshot_value, roots.file, roots.generated_statics_fstring,
        roots.class_descriptor_shared, roots.has_statics, projected);
    if (frame_status != TargetFrontendRawError::ok) {
      live_capture_note_failure_detail_v1(
          0xB610u + static_cast<std::uint32_t>(frame_status));
      return false;
    }
    if (before) {
      TargetFrontendNativeSuperRaw native{};
      const auto native_status = materialize_class_native_super_v1(
          snapshot_value, roots.class_descriptor_shared, native);
      if (native_status != TargetFrontendRawError::ok) {
        live_capture_note_failure_detail_v1(
            0xB630u + static_cast<std::uint32_t>(native_status));
        return false;
      }
      if (native.present) {
        FrontendNativeSuper semantic{};
        const auto derive_status = derive_native_super_v1(native.witness, semantic);
        if (derive_status != FrontendObserverError::ok) {
          live_capture_note_failure_detail_v1(
              0xB650u + static_cast<std::uint32_t>(derive_status));
          return false;
        }
        native_supers.push_back(std::move(semantic));
      }
      const auto begin_status = frontend.begin_class_analyze(projected);
      if (begin_status != FrontendObserverError::ok) {
        live_capture_note_failure_detail_v1(
            0xB670u + static_cast<std::uint32_t>(begin_status));
        return false;
      }
      class_pending_roots = roots;
      class_pending = true;
      return true;
    }
    const auto complete_status = frontend.complete_class_analyze(projected);
    if (complete_status != FrontendObserverError::ok) {
      live_capture_note_failure_detail_v1(
          0xB690u + static_cast<std::uint32_t>(complete_status));
      return false;
    }
    class_pending_roots = {};
    class_pending = false;
    return true;
  }

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  bool fixture_observe(
      const std::uint32_t site,
      const ProductionShimPhase phase) {
    if (fixture_next >= fixture_events.size() ||
        fixture_events[fixture_next] != std::pair{site, phase}) {
      return false;
    }
    ++fixture_next;
    PublicRegistrySnapshot snapshot_value{};
    snapshot_value.canonical_sha256[0] = std::byte{1};
    if (site == 0) {
      return machine.add_engine_property(1, 2) == ProductionCapturePhaseError::ok;
    }
    if (site == 1) {
      std::uint32_t token = 0;
      return machine.intern_primary_image_pointer(image + 0x1000, token) ==
                 ProductionCapturePhaseError::ok &&
             machine.begin_bind(3, token, snapshot_value) ==
                 ProductionCapturePhaseError::ok;
    }
    if (site >= 9 && site <= 22 && phase == ProductionShimPhase::after) {
      return machine.add_registry_delta("{}") == ProductionCapturePhaseError::ok;
    }
    if (site == 2) {
      return machine.end_bind(snapshot_value) == ProductionCapturePhaseError::ok &&
             machine.complete_registry("{}", {"{}"}) ==
                 ProductionCapturePhaseError::ok;
    }
    if (site == 4 && phase == ProductionShimPhase::after) {
      gore_as_capture_build_jit_v1 build{};
      build.struct_size = sizeof(build);
      build.build_identifier = kBuildIdentifier;
      build.shipping_cache_matches = 1;
      build.fork_opcode_table_201_212_present = 1;
      return machine.set_build_jit(build) == ProductionCapturePhaseError::ok;
    }
    if (site == 23 || site == 24) {
      const bool before = phase == ProductionShimPhase::before;
      if (fixture_binding_drift ||
          (site == 23 &&
           ((before &&
             (process_pending || process_complete || post_pending || post_complete)) ||
            (!before &&
             (!process_pending || process_complete || post_pending || post_complete)))) ||
          (site == 24 &&
           ((before &&
             (!process_complete || process_pending || post_pending || post_complete)) ||
            (!before &&
             (!process_complete || process_pending || !post_pending || post_complete))))) {
        return false;
      }
      if (site == 23) {
        process_pending = before;
        process_complete = !before;
      } else {
        post_pending = before;
        post_complete = !before;
      }
      return true;
    }
    if (site == 25) {
      if (phase == ProductionShimPhase::before) {
        if (class_pending) return false;
        class_pending_roots.phase = TargetFrontendSnapshotPhase::class_analyze;
        class_pending_roots.file = 0x1111;
        class_pending_roots.generated_statics_fstring = 0x2222;
        class_pending_roots.class_descriptor_shared = 0x3333;
        class_pending_roots.has_statics = 0x4444;
        class_pending = true;
        return true;
      }
      if (!class_pending ||
          class_pending_roots.phase != TargetFrontendSnapshotPhase::class_analyze ||
          class_pending_roots.file != 0x1111 ||
          class_pending_roots.generated_statics_fstring != 0x2222 ||
          class_pending_roots.class_descriptor_shared != 0x3333 ||
          class_pending_roots.has_statics != 0x4444) {
        return false;
      }
      class_pending_roots = {};
      class_pending = false;
      return true;
    }
    if (site == 8) {
      if (process_pending || post_pending || class_pending ||
          !process_complete || !post_complete) return false;
      FrontendDigest digest{};
      digest[0] = 1;
      std::vector<FrontendBoundaryProjection> boundaries{
          {FrontendBoundaryKind::initial_compile_enter, kRvaInitialCompileEnter, 0, 0,
           digest, {}, {}},
          {FrontendBoundaryKind::preprocessor_constructed,
           kRvaPreprocessorConstructed, 0, 0, digest, {}, {}},
          {FrontendBoundaryKind::initial_compile_return,
           kRvaInitialCompileReturn, 1, 0, digest, digest, digest}};
      return machine.set_frontend("{}", "{}", "{}", std::move(boundaries)) ==
                 ProductionCapturePhaseError::ok &&
             machine.complete() == ProductionCapturePhaseError::ok;
    }
    return true;
  }
#endif

  bool finalize_frontend(const ProductionMachineFrame& frame);
  bool capture_complete_native_supers() noexcept;
  bool observe(std::uint32_t site, ProductionShimPhase phase, ProductionMachineFrame& frame);

  ProductionCapturePhaseMachine machine;
  ProductionPatchCoordinator patches;
  std::unique_ptr<TargetRegistrationObserver> registrations;
  FrontendSemanticObserver frontend;
  std::vector<PendingRegistration> pending;
  // These catalogs intentionally retain only process-local addresses long enough to assign
  // deterministic, class-local provenance tokens. No address is serialized or replayed.
  std::vector<std::uintptr_t> object_pointers;
  std::vector<std::uintptr_t> storage_pointers;
  std::vector<FrontendNativeSuper> native_supers;
  std::vector<FrontendGraphModule> descriptor_modules;
  BindMetadata bind{};
  PublicRegistrySnapshot registry_snapshot{};
  TargetFrontendGraphHookBindings hook_bindings{};
  std::uintptr_t image{};
  std::uintptr_t manager{};
  std::uintptr_t engine{};
  std::uintptr_t preprocessor{};
  std::uintptr_t saved_boundary_rcx{};
  std::uintptr_t saved_boundary_rdx{};
  std::uintptr_t jit_pointer{};
  std::uint64_t session_id{};
  std::uint64_t epoch{};
  std::uint32_t owner_thread{};
  std::atomic<std::uint32_t> semantic_thread{};
  std::uint32_t build_identifier{};
  std::uint32_t bind_ordinal{};
  FrontendBoundaryKind middle{};
  bool active_bind{};
  bool synthetic_bind{};
  bool registry_complete{};
  bool have_build_id{};
  bool have_jit{};
  bool build_emitted{};
  bool frontend_emitted{};
  bool frontend_started{};
  bool process_pending{};
  bool process_complete{};
  bool post_pending{};
  bool post_complete{};
  bool class_pending{};
  TargetFrontendSnapshotRoots class_pending_roots{};
  bool final_frontend_snapshot_ready{};
  TargetFrontendSnapshot final_frontend_snapshot;
  bool preflighted{};
  bool failed{};
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  bool fixture_mode{};
  bool fixture_binding_drift{};
  std::size_t fixture_next{};
  std::vector<std::pair<std::uint32_t, ProductionShimPhase>> fixture_events;
#endif
};

bool ProductionCaptureCoordinator::Impl::capture_complete_native_supers() noexcept {
  std::vector<NativeClassCapability> capabilities;
  const auto captured = capture_native_class_capabilities_v23300(
      image, kPeSizeOfImage, engine, capabilities);
  if (captured != SnapshotError::ok) {
    live_capture_note_failure_detail_v1(
        0xB800u + static_cast<std::uint32_t>(captured));
    return false;
  }
  std::uint64_t snapshots_built = 0;
  std::uint64_t class_paths_materialized = 0;
  std::uint64_t uclass_matches = 0;
  std::uint32_t first_class_path_error = 0;
  std::uint64_t first_class_fname = 0;
  std::uint64_t first_class_entry = 0;
  std::string sample_object_class_path;
  for (const auto& capability : capabilities) {
    if (!capability.name_space.empty()) continue;
    if (++epoch == 0) return false;
    TargetFrontendSnapshotRoots roots{};
    roots.phase = TargetFrontendSnapshotPhase::native_class;
    roots.uclass = capability.user_data;
    TargetFrontendSnapshot native_snapshot;
    const auto built = build_current_process_frontend_snapshot_v1(
        image, epoch, roots, native_snapshot);
    // Non-UObject user data is valid AngelScript host state, but is not a native superclass.
    if (built != TargetFrontendSnapshotBuildError::ok) continue;
    ++snapshots_built;
    std::string object_class_path;
    const auto class_path_status = materialize_uobject_class_path_v1(
        native_snapshot, capability.user_data, object_class_path);
    if (class_path_status != TargetFrontendRawError::ok) {
      if (first_class_path_error == 0) {
        first_class_path_error = static_cast<std::uint32_t>(class_path_status);
        std::uintptr_t object_class = 0;
        std::uintptr_t object_outer = 0;
        TargetRawFName raw_name{};
        std::uintptr_t block = 0;
        // The direct "Class" entry was proven by the preceding diagnostic run. Capture the
        // outer package entry now: its two-byte header plus six leading bytes fit one scalar.
        std::array<std::byte, 8> entry_bytes{};
        if (native_snapshot.read(
                capability.user_data + frontend_target_layout::uobject_class,
                {reinterpret_cast<std::byte*>(&object_class), sizeof(object_class)},
                TargetRawRegionKind::immutable_data) == TargetFrontendRawError::ok &&
            native_snapshot.read(
                object_class + frontend_target_layout::uobject_outer,
                {reinterpret_cast<std::byte*>(&object_outer), sizeof(object_outer)},
                TargetRawRegionKind::immutable_data) == TargetFrontendRawError::ok &&
            native_snapshot.read(
                object_outer + frontend_target_layout::uobject_name,
                {reinterpret_cast<std::byte*>(&raw_name), sizeof(raw_name)},
                TargetRawRegionKind::immutable_data) == TargetFrontendRawError::ok &&
            native_snapshot.read(
                image + frontend_target_layout::fname_pool_rva + 0x10u +
                    static_cast<std::size_t>(raw_name.comparison_index >> 16) *
                        sizeof(block),
                {reinterpret_cast<std::byte*>(&block), sizeof(block)},
                TargetRawRegionKind::primary_image) == TargetFrontendRawError::ok &&
            native_snapshot.read(
                block + static_cast<std::size_t>(raw_name.comparison_index & 0xffffu) * 2u,
                entry_bytes, TargetRawRegionKind::immutable_data) ==
                TargetFrontendRawError::ok) {
          first_class_fname =
              static_cast<std::uint64_t>(raw_name.comparison_index) |
              (static_cast<std::uint64_t>(raw_name.number) << 32);
          std::memcpy(&first_class_entry, entry_bytes.data(), sizeof(first_class_entry));
        }
      }
      continue;
    }
    ++class_paths_materialized;
    if (sample_object_class_path.empty()) sample_object_class_path = object_class_path;
    if (object_class_path != "/Script/CoreUObject.Class") continue;
    ++uclass_matches;
    FrontendNativeClassWitness witness{};
    if (materialize_native_class_witness_v1(
            native_snapshot, capability.user_data,
            capability.angelscript_type_name, witness) !=
        TargetFrontendRawError::ok) {
      return false;
    }
    FrontendNativeSuper semantic{};
    if (derive_native_super_v1(witness, semantic) != FrontendObserverError::ok) {
      return false;
    }
    try {
      native_supers.push_back(std::move(semantic));
    } catch (...) {
      return false;
    }
  }
  live_capture_note_registration_arguments_v1(
      "native_catalog", 14, nullptr, 0,
      static_cast<std::uint64_t>(capabilities.size()), snapshots_built,
      class_paths_materialized);
  if (uclass_matches == 0 || native_supers.empty()) {
    live_capture_note_registration_arguments_v1(
        "native_uclasses", 15,
        sample_object_class_path.empty() ? nullptr : sample_object_class_path.data(),
        static_cast<std::uint32_t>(sample_object_class_path.size()),
        first_class_fname, first_class_entry, first_class_path_error);
    return false;
  }
  return true;
}

bool ProductionCaptureCoordinator::Impl::finalize_frontend(
    const ProductionMachineFrame& frame) {
  const auto reject = [](const std::uint32_t detail) {
    live_capture_note_failure_detail_v1(detail);
    return false;
  };
  if (!frontend_started || middle == FrontendBoundaryKind{} || frame.rbx != manager ||
      !registry_complete || process_pending || post_pending || class_pending ||
      !process_complete || !post_complete) {
    const std::uint64_t state =
        (frontend_started ? 1ull << 0 : 0) |
        (static_cast<std::uint64_t>(middle) << 1) |
        (frame.rbx == manager ? 1ull << 8 : 0) |
        (registry_complete ? 1ull << 9 : 0) |
        (build_emitted ? 1ull << 10 : 0) |
        (process_pending ? 1ull << 11 : 0) |
        (post_pending ? 1ull << 12 : 0) |
        (process_complete ? 1ull << 13 : 0) |
        (post_complete ? 1ull << 14 : 0) |
        (class_pending ? 1ull << 15 : 0);
    live_capture_note_registration_arguments_v1(
        "finalize_state", 14, nullptr, 0, state, frame.rbx, manager);
    return reject(0xB601u);
  }
  std::uint8_t succeeded = 0;
  if (!read_current_value(manager + kManagerInitialCompileSucceededOffset, succeeded) ||
      succeeded != 1) {
    return reject(0xB602u);
  }

  TargetFrontendSnapshot snapshot_value;
  const TargetFrontendSnapshot* captured_snapshot = nullptr;
  TargetFrontendSnapshotRoots roots{};
  roots.manager = manager;
  if (middle == FrontendBoundaryKind::preprocessor_constructed) {
    if (!final_frontend_snapshot_ready) return reject(0xB609u);
    captured_snapshot = &final_frontend_snapshot;
  } else {
    roots.phase = TargetFrontendSnapshotPhase::settings_configuration;
    if (!snapshot(roots, snapshot_value)) return reject(0xB603u);
    captured_snapshot = &snapshot_value;
  }
  const auto& final_snapshot = *captured_snapshot;

  std::uintptr_t settings = 0;
  if (final_snapshot.read_any(
          manager + frontend_target_layout::manager_settings,
          {reinterpret_cast<std::byte*>(&settings), sizeof(settings)}) !=
          TargetFrontendRawError::ok ||
      settings == 0) {
    return reject(0xB604u);
  }
  std::array<std::byte, kSettingsBytes> settings_bytes{};
  std::array<std::byte, kPreprocessorBytes> preprocessor_bytes{};
  if (final_snapshot.read(
          settings, settings_bytes, TargetRawRegionKind::immutable_data) !=
      TargetFrontendRawError::ok) {
    return reject(0xB605u);
  }
  if (middle == FrontendBoundaryKind::preprocessor_constructed) {
    if (final_snapshot.read(
            preprocessor, preprocessor_bytes, TargetRawRegionKind::immutable_data) !=
        TargetFrontendRawError::ok) {
      return reject(0xB606u);
    }
  } else {
    // RVA 0x4885800 initializes these four effective preprocessor bytes from the settings
    // defaults. The precompiled-descriptor CFG skips construction, so reproduce only those
    // witnessed scalar copies; all containers are materialized from their actual settings root.
    preprocessor_bytes[frontend_target_layout::preprocessor_default_function_blueprint] =
        settings_bytes[frontend_target_layout::settings_default_function_blueprint];
    preprocessor_bytes[frontend_target_layout::preprocessor_default_property_edit] =
        settings_bytes[frontend_target_layout::settings_default_property_edit];
    preprocessor_bytes[frontend_target_layout::preprocessor_default_struct_property_edit] =
        settings_bytes[frontend_target_layout::settings_default_struct_property_edit];
    preprocessor_bytes[frontend_target_layout::preprocessor_default_property_blueprint] =
        settings_bytes[frontend_target_layout::settings_default_property_blueprint];
  }

  bool automatic_imports = false;
  bool use_editor_scripts = false;
  std::uint8_t raw = 0;
  if (final_snapshot.read(
          image + frontend_target_layout::automatic_imports_rva,
          {reinterpret_cast<std::byte*>(&raw), sizeof(raw)},
          TargetRawRegionKind::primary_image) != TargetFrontendRawError::ok ||
      raw > 1) {
    return reject(0xB607u);
  }
  automatic_imports = raw != 0;
  if (final_snapshot.read(
          image + frontend_target_layout::use_editor_scripts_rva,
          {reinterpret_cast<std::byte*>(&raw), sizeof(raw)},
          TargetRawRegionKind::primary_image) != TargetFrontendRawError::ok ||
      raw > 1) {
    return reject(0xB608u);
  }
  use_editor_scripts = raw != 0;

  TargetFrontendGraphHookBindings final_bindings{};
  FrontendPreprocessorConfig config{};
  const auto final_binding_status =
      materialize_graph_hook_bindings_v1(final_snapshot, final_bindings);
  if (final_binding_status != TargetFrontendRawError::ok) {
    return reject(0xB610u + static_cast<std::uint32_t>(final_binding_status));
  }
  if (
      final_bindings.class_analyze_bound != hook_bindings.class_analyze_bound ||
      final_bindings.class_analyze_active_bindings !=
          hook_bindings.class_analyze_active_bindings ||
      final_bindings.process_chunks_bound || final_bindings.post_process_code_bound) {
    return reject(0xB620u);
  }
  const auto hook_config_status = materialize_graph_hook_config_v1(final_snapshot, config);
  if (hook_config_status != TargetFrontendRawError::ok) {
    return reject(0xB630u + static_cast<std::uint32_t>(hook_config_status));
  }
  const auto flag_status = middle == FrontendBoundaryKind::preprocessor_constructed
                               ? materialize_preprocessor_flags_v1(
                                     final_snapshot, preprocessor, config.effective_flags)
                               : materialize_settings_flags_v1(
                                     final_snapshot, manager, config.effective_flags);
  if (flag_status != TargetFrontendRawError::ok) {
    return reject(0xB640u + static_cast<std::uint32_t>(flag_status));
  }
  if (!target_flag_set(config.effective_flags)) return reject(0xB650u);
  const auto specialization_status = materialize_blueprint_specializations_v1(
      final_snapshot, manager, config.blueprint_event_argument_specializations);
  if (specialization_status != TargetFrontendRawError::ok) {
    return reject(0xB660u + static_cast<std::uint32_t>(specialization_status));
  }
  const auto fname_status =
      materialize_static_fnames_v1(final_snapshot, config.fname_comparison_keys);
  if (fname_status != TargetFrontendRawError::ok) {
    return reject(0xB670u + static_cast<std::uint32_t>(fname_status));
  }
  if (!capture_complete_native_supers()) return reject(0xB67Fu);
  std::sort(native_supers.begin(), native_supers.end(), [](const auto& left, const auto& right) {
    return left.angelscript_type_name < right.angelscript_type_name;
  });
  std::vector<FrontendNativeSuper> unique_supers;
  for (auto& value : native_supers) {
    if (!unique_supers.empty() &&
        unique_supers.back().angelscript_type_name == value.angelscript_type_name) {
      const auto& previous = unique_supers.back();
      if (previous.unreal_class_path != value.unreal_class_path ||
          previous.property_offset != value.property_offset || previous.kind != value.kind ||
          previous.game_state_subsystem != value.game_state_subsystem ||
          previous.cannot_derive_angelscript != value.cannot_derive_angelscript) {
        return reject(0xB680u);
      }
      continue;
    }
    unique_supers.push_back(std::move(value));
  }
  config.native_super_types = std::move(unique_supers);
  const auto frontend_status = frontend.finish(config);
  if (frontend_status != FrontendObserverError::ok) {
    return reject(0xB690u + static_cast<std::uint32_t>(frontend_status));
  }

  FrontendClassGeneratorConfig generator{};
  FrontendCompilerOptions options{};
  const auto settings_status = project_frontend_settings_v1(
          settings_bytes.data(), settings_bytes.size(), preprocessor_bytes.data(),
          preprocessor_bytes.size(), automatic_imports, use_editor_scripts, config,
          generator, options);
  if (settings_status != FrontendObserverError::ok) {
    return reject(0xB6A0u + static_cast<std::uint32_t>(settings_status));
  }
  std::string preprocessor_json;
  std::string generator_json;
  std::string options_json;
  auto serialize_status = serialize_preprocessor_config_json_v1(config, preprocessor_json);
  if (serialize_status != FrontendObserverError::ok) {
    return reject(0xB6B0u + static_cast<std::uint32_t>(serialize_status));
  }
  serialize_status = serialize_class_generator_config_json_v1(generator, generator_json);
  if (serialize_status != FrontendObserverError::ok) {
    return reject(0xB6C0u + static_cast<std::uint32_t>(serialize_status));
  }
  serialize_status = serialize_compiler_options_json_v1(options, options_json);
  if (serialize_status != FrontendObserverError::ok) {
    return reject(0xB6D0u + static_cast<std::uint32_t>(serialize_status));
  }

  FrontendDigest config_digest{};
  if (frontend_config_set_digest_v1(config, generator, options, config_digest) !=
      FrontendObserverError::ok) {
    return reject(0xB6E0u);
  }
  std::vector<FrontendGraphModule> final_modules;
  if (middle == FrontendBoundaryKind::precompiled_descriptors_requested) {
    final_modules = descriptor_modules;
  } else {
    const auto graph_status = materialize_preprocessor_graph_v1(
        final_snapshot, preprocessor, TargetFrontendGraphSource::processed_code,
        final_modules);
    if (graph_status != TargetFrontendRawError::ok) {
      return reject(0xB700u + static_cast<std::uint32_t>(graph_status));
    }
    if (final_modules.empty()) return reject(0xB710u);
  }
  std::vector<FrontendBoundaryProjection> boundaries(3);
  if (project_initial_compile_enter_v1(config_digest, boundaries[0]) !=
      FrontendObserverError::ok) {
    return reject(0xB720u);
  }
  if (middle == FrontendBoundaryKind::precompiled_descriptors_requested) {
    if (project_precompiled_descriptors_v1(
            config_digest, descriptor_modules, boundaries[1]) != FrontendObserverError::ok) {
      return reject(0xB730u);
    }
  } else if (project_preprocessor_constructed_v1(config_digest, boundaries[1]) !=
             FrontendObserverError::ok) {
    return reject(0xB740u);
  }
  const auto return_status =
      project_initial_compile_return_v1(config_digest, final_modules, boundaries[2]);
  if (return_status != FrontendObserverError::ok) {
    return reject(0xB750u + static_cast<std::uint32_t>(return_status));
  }
  const auto machine_frontend_status = machine.set_frontend(
      std::move(preprocessor_json), std::move(generator_json),
      std::move(options_json), std::move(boundaries));
  if (machine_frontend_status != ProductionCapturePhaseError::ok) {
    return reject(0xB760u + static_cast<std::uint32_t>(machine_frontend_status));
  }
  frontend_emitted = true;
  if (!build_emitted) {
    if (!have_build_id) {
      build_identifier = kBuildIdentifier;
      have_build_id = true;
    }
    if (!have_jit) {
      if (!read_current_value(image + kStaticJitInfoStorageRva, jit_pointer)) {
        return reject(0xB780u);
      }
      have_jit = true;
    }
    if (!try_build_jit() || !build_emitted || !machine.committed()) {
      return reject(0xB781u);
    }
    return true;
  }
  const auto complete_status = machine.complete();
  if (complete_status != ProductionCapturePhaseError::ok) {
    return reject(0xB770u + static_cast<std::uint32_t>(complete_status));
  }
  return true;
}

bool ProductionCaptureCoordinator::Impl::observe(
    const std::uint32_t site,
    const ProductionShimPhase phase,
    ProductionMachineFrame& frame) {
  if (!preflighted || failed || site >= kProductionSiteCount) return fail();
#if defined(GORE_AS_CAPTURE_TEST_TARGET)
  if (fixture_mode) return fixture_observe(site, phase) || fail();
#endif
  const bool registration_site =
      site >= kProductionBaseSiteCount &&
      site < kProductionBaseSiteCount + kProductionRegistrationSiteCount;
  const auto current_thread = GetCurrentThreadId();
  auto semantic_owner = semantic_thread.load(std::memory_order_acquire);
  if (semantic_owner == 0) {
    const bool ownership_anchor =
        phase == ProductionShimPhase::before && (site == 0 || registration_site);
    if (!ownership_anchor ||
        !semantic_thread.compare_exchange_strong(
            semantic_owner, current_thread, std::memory_order_acq_rel) ||
        !bridge_adopt_runtime_owner_v1(session_id, image) ||
        machine.adopt_runtime_owner() != ProductionCapturePhaseError::ok) {
      return fail();
    }
    semantic_owner = current_thread;
  }
  if (semantic_owner != current_thread) return fail();
  if (registration_site) {
    return (phase == ProductionShimPhase::before ? registration_before(site, frame)
                                                 : registration_after(site, frame)) ||
           fail();
  }
  if (site >= 23) {
    bool accepted = false;
    if (site == 23 || site == 24) {
      accepted = graph(site, phase, frame);
    } else {
      accepted = class_analyze(phase, frame);
    }
    return accepted || fail();
  }

  bool accepted = false;
  switch (site) {
    case 0:
      accepted = phase == ProductionShimPhase::before &&
                 machine.add_engine_property(
                     static_cast<std::uint32_t>(frame.rdx), frame.r8) ==
                     ProductionCapturePhaseError::ok;
      break;
    case 1:
      accepted = phase == ProductionShimPhase::before && begin_bind(frame);
      break;
    case 2:
      accepted = phase == ProductionShimPhase::before && end_bind(frame);
      break;
    case 3:
      if (phase == ProductionShimPhase::before) {
        accepted = true;
      } else {
        build_identifier = static_cast<std::uint32_t>(frame.rax);
        have_build_id = true;
        accepted = try_build_jit();
      }
      break;
    case 4:
      if (phase == ProductionShimPhase::before) {
        accepted = true;
      } else {
        jit_pointer = frame.rax;
        have_jit = true;
        accepted = try_build_jit();
      }
      break;
    case 5:
      accepted = phase == ProductionShimPhase::before && begin_frontend(frame);
      break;
    case 6:
      if (phase == ProductionShimPhase::before) {
        accepted = saved_boundary_rcx == 0 && saved_boundary_rdx == 0;
        if (accepted) {
          saved_boundary_rcx = frame.rcx;
          saved_boundary_rdx = frame.rdx;
        }
      } else {
        accepted = descriptor_after(frame);
        saved_boundary_rcx = 0;
        saved_boundary_rdx = 0;
      }
      break;
    case 7:
      if (phase == ProductionShimPhase::before) {
        accepted = saved_boundary_rcx == 0 && frame.rcx != 0;
        if (accepted) saved_boundary_rcx = frame.rcx;
      } else {
        accepted = preprocessor_after(frame);
        saved_boundary_rcx = 0;
      }
      break;
    case 8:
      accepted = phase == ProductionShimPhase::before && finalize_frontend(frame);
      break;
    default:
      accepted = false;
      break;
  }
  return accepted || fail();
}

ProductionCaptureCoordinator::ProductionCaptureCoordinator() noexcept = default;

ProductionCaptureCoordinator::~ProductionCaptureCoordinator() {
  if (impl_ != nullptr && !impl_->patches.installed() && !impl_->machine.needs_abort()) {
    delete impl_;
  }
}

ProductionCaptureCoordinatorError ProductionCaptureCoordinator::preflight(
    const std::uint64_t session_id,
    const std::uintptr_t primary_image,
    const ProductionCaptureSink sink) noexcept {
  if (impl_ != nullptr || session_id == 0 || primary_image == 0) {
    return ProductionCaptureCoordinatorError::invalid_state;
  }
  auto* const state = new (std::nothrow) Impl;
  if (state == nullptr) return ProductionCaptureCoordinatorError::terminal_failure;
  state->image = primary_image;
  state->session_id = session_id;
  state->owner_thread = GetCurrentThreadId();
  if (state->machine.preflight(session_id, primary_image, sink) !=
          ProductionCapturePhaseError::ok ||
      state->patches.preflight(
          primary_image, session_id,
          ProductionShimObserver{state, ProductionCaptureCoordinator::dispatch}) !=
          ProductionPatchError::ok) {
    if (state->machine.needs_abort()) (void)state->machine.abort();
    if (state->machine.needs_abort()) {
      state->failed = true;
      impl_ = state;
      return ProductionCaptureCoordinatorError::recovery_required;
    }
    delete state;
    return ProductionCaptureCoordinatorError::target_drift;
  }
  state->preflighted = true;
  impl_ = state;
  return ProductionCaptureCoordinatorError::ok;
}

ProductionCaptureCoordinatorError ProductionCaptureCoordinator::install() noexcept {
  if (impl_ == nullptr || !impl_->preflighted || impl_->failed) {
    return ProductionCaptureCoordinatorError::invalid_state;
  }
  const auto result = impl_->patches.install();
  if (result == ProductionPatchError::ok) return ProductionCaptureCoordinatorError::ok;
  if (impl_->machine.needs_abort()) (void)impl_->machine.abort();
  impl_->failed = true;
  if (impl_->machine.needs_abort()) {
    return ProductionCaptureCoordinatorError::recovery_required;
  }
  if (result == ProductionPatchError::rollback_failed && impl_->patches.installed()) {
    return ProductionCaptureCoordinatorError::recovery_required;
  }
  return ProductionCaptureCoordinatorError::patch_failure;
}

ProductionCaptureCoordinatorError ProductionCaptureCoordinator::uninstall() noexcept {
  if (impl_ == nullptr ||
      (!impl_->patches.installed() && !impl_->machine.needs_abort())) {
    return ProductionCaptureCoordinatorError::invalid_state;
  }
  if (!impl_->owner()) return ProductionCaptureCoordinatorError::wrong_thread;
  if (impl_->machine.needs_abort()) {
    if (impl_->machine.abort() != ProductionCapturePhaseError::ok) {
      return ProductionCaptureCoordinatorError::terminal_failure;
    }
  }
  if (impl_->patches.installed()) {
    const auto result = impl_->patches.uninstall();
    if (result != ProductionPatchError::ok) {
      if (result == ProductionPatchError::rollback_failed) {
        return ProductionCaptureCoordinatorError::recovery_required;
      }
      return ProductionCaptureCoordinatorError::patch_failure;
    }
  }
  delete impl_;
  impl_ = nullptr;
  return ProductionCaptureCoordinatorError::ok;
}

ProductionCaptureCoordinatorError ProductionCaptureCoordinator::prepare_unload() const noexcept {
  if (impl_ == nullptr) return ProductionCaptureCoordinatorError::ok;
  if (impl_->machine.needs_abort()) return ProductionCaptureCoordinatorError::terminal_failure;
  return impl_->patches.prepare_unload() == ProductionPatchError::ok
             ? ProductionCaptureCoordinatorError::ok
             : ProductionCaptureCoordinatorError::patch_failure;
}

bool ProductionCaptureCoordinator::installed() const noexcept {
  return impl_ != nullptr && impl_->patches.installed();
}

bool ProductionCaptureCoordinator::recovery_required() const noexcept {
  return impl_ != nullptr &&
         (impl_->patches.installed() || impl_->machine.needs_abort());
}

bool ProductionCaptureCoordinator::committed() const noexcept {
  return impl_ != nullptr && impl_->machine.committed();
}

bool ProductionCaptureCoordinator::terminal() const noexcept {
  return impl_ != nullptr && (impl_->failed || impl_->machine.terminal());
}

bool ProductionCaptureCoordinator::dispatch(
    void* const context,
    const std::uint32_t site_id,
    const ProductionShimPhase phase,
    ProductionMachineFrame& frame) noexcept {
  LARGE_INTEGER started{};
  LARGE_INTEGER finished{};
  (void)QueryPerformanceCounter(&started);
  const auto note_timing = [&]() noexcept {
    if (QueryPerformanceCounter(&finished) != FALSE &&
        finished.QuadPart >= started.QuadPart) {
      live_capture_note_dispatch_timing_v1(
          static_cast<std::uint64_t>(finished.QuadPart - started.QuadPart));
    }
  };
  try {
    auto* const state = static_cast<Impl*>(context);
    const bool accepted = state != nullptr && state->observe(site_id, phase, frame);
    if (!accepted) {
      live_capture_note_dispatch_failure_v1(
          site_id, static_cast<std::uint32_t>(phase));
    }
    note_timing();
    return accepted;
  } catch (...) {
    auto* const state = static_cast<Impl*>(context);
    const bool accepted = state != nullptr ? state->fail() : false;
    live_capture_note_dispatch_failure_v1(
        site_id, static_cast<std::uint32_t>(phase));
    note_timing();
    return accepted;
  }
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {

class DispatcherSyntheticImage final {
 public:
  DispatcherSyntheticImage() noexcept
      : image_(static_cast<std::byte*>(
            VirtualAlloc(nullptr, kPeSizeOfImage, MEM_RESERVE, PAGE_NOACCESS))) {
    if (image_ == nullptr) return;
    SYSTEM_INFO system{};
    GetSystemInfo(&system);
    page_bytes_ = system.dwPageSize;
    if (page_bytes_ == 0) return;
    for (const auto& site : kPinnedInstructionSpans) {
      if (!copy(site.patch_anchor_rva, site.expected.data(), site.byte_count)) return;
    }
    for (const auto& site : registration::kPinnedRegistrationHooks) {
      if (!copy(site.function_rva, site.expected.data(), site.overwrite_bytes)) return;
    }
    for (const auto& site : frontend_target_layout::callback_callsites) {
      if (!copy(site.call_rva, site.expected_call.data(), site.expected_call.size())) return;
    }
    for (const auto page : pages_) {
      DWORD ignored = 0;
      if (VirtualProtect(image_ + page, page_bytes_, PAGE_EXECUTE_READ, &ignored) == FALSE) {
        return;
      }
    }
    ready_ = true;
  }

  ~DispatcherSyntheticImage() {
    if (image_ != nullptr) (void)VirtualFree(image_, 0, MEM_RELEASE);
  }

  std::uintptr_t address() const noexcept {
    return reinterpret_cast<std::uintptr_t>(image_);
  }
  bool ready() const noexcept { return ready_; }

 private:
  bool commit(const std::uint32_t rva, const std::size_t bytes) {
    if (rva >= kPeSizeOfImage || bytes == 0 || bytes > kPeSizeOfImage - rva) return false;
    const auto first = static_cast<std::uint32_t>(rva - rva % page_bytes_);
    const auto last = static_cast<std::uint32_t>(
        (rva + bytes - 1) - (rva + bytes - 1) % page_bytes_);
    for (std::uint32_t page = first;; page += page_bytes_) {
      if (std::find(pages_.begin(), pages_.end(), page) == pages_.end()) {
        if (VirtualAlloc(image_ + page, page_bytes_, MEM_COMMIT, PAGE_READWRITE) !=
            image_ + page) {
          return false;
        }
        pages_.push_back(page);
      }
      if (page == last) break;
      if (page > std::numeric_limits<std::uint32_t>::max() - page_bytes_) return false;
    }
    return true;
  }

  bool copy(
      const std::uint32_t rva,
      const std::byte* const bytes,
      const std::size_t count) {
    if (!commit(rva, count)) return false;
    std::memcpy(image_ + rva, bytes, count);
    return true;
  }

  std::byte* image_{};
  std::uint32_t page_bytes_{};
  std::vector<std::uint32_t> pages_;
  bool ready_{};
};

struct DispatcherFixtureSink final {
  std::uintptr_t image{};
  std::uint32_t pointer_tokens{};
  std::uint32_t appends{};
  std::uint32_t seals{};
  std::uint32_t aborts{};
  std::uint32_t abort_failures{};
};

std::uint32_t fixture_append(DispatcherFixtureSink& sink) noexcept {
  ++sink.appends;
  return GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

ProductionCaptureSink dispatcher_fixture_sink(DispatcherFixtureSink& sink) noexcept {
  return {
      &sink,
      [](void* context, std::uint64_t session, std::uintptr_t image) noexcept {
        const auto& value = *static_cast<DispatcherFixtureSink*>(context);
        return session == 77 && image == value.image;
      },
      [](void* context, std::uint64_t, std::uintptr_t, std::uint32_t& token) noexcept {
        auto& value = *static_cast<DispatcherFixtureSink*>(context);
        token = value.pointer_tokens++;
        return fixture_append(value);
      },
      [](void* context, std::uint64_t, std::uint32_t, std::uint64_t) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, std::uint32_t, std::int32_t, std::uint32_t,
         const PublicRegistrySnapshot&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, std::uint32_t, std::int32_t, std::uint32_t,
         const PublicRegistrySnapshot&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, std::uint32_t, const std::string&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, const gore_as_capture_build_jit_v1&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, std::uint32_t, const std::string&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t, const FrontendBoundaryProjection&) noexcept {
        return fixture_append(*static_cast<DispatcherFixtureSink*>(context));
      },
      [](void* context, std::uint64_t) noexcept -> std::uint32_t {
        auto& value = *static_cast<DispatcherFixtureSink*>(context);
        ++value.seals;
        return GORE_AS_CAPTURE_BRIDGE_OK_V1;
      },
      [](void* context, std::uint64_t) noexcept -> std::uint32_t {
        auto& value = *static_cast<DispatcherFixtureSink*>(context);
        ++value.aborts;
        if (value.abort_failures != 0) {
          --value.abort_failures;
          return GORE_AS_CAPTURE_BRIDGE_IO_ERROR_V1;
        }
        return GORE_AS_CAPTURE_BRIDGE_OK_V1;
      }};
}

std::vector<std::pair<std::uint32_t, ProductionShimPhase>> fixture_events(
    const std::uint32_t middle_site) {
  std::vector<std::pair<std::uint32_t, ProductionShimPhase>> events;
  events.emplace_back(0, ProductionShimPhase::before);
  events.emplace_back(1, ProductionShimPhase::before);
  for (std::uint32_t site = 9; site <= 22; ++site) {
    events.emplace_back(site, ProductionShimPhase::before);
    events.emplace_back(site, ProductionShimPhase::after);
  }
  events.emplace_back(2, ProductionShimPhase::before);
  events.emplace_back(3, ProductionShimPhase::before);
  events.emplace_back(3, ProductionShimPhase::after);
  events.emplace_back(4, ProductionShimPhase::before);
  events.emplace_back(4, ProductionShimPhase::after);
  events.emplace_back(5, ProductionShimPhase::before);
  events.emplace_back(middle_site, ProductionShimPhase::before);
  events.emplace_back(middle_site, ProductionShimPhase::after);
  events.emplace_back(23, ProductionShimPhase::before);
  events.emplace_back(23, ProductionShimPhase::after);
  events.emplace_back(24, ProductionShimPhase::before);
  events.emplace_back(24, ProductionShimPhase::after);
  events.emplace_back(25, ProductionShimPhase::before);
  events.emplace_back(25, ProductionShimPhase::after);
  events.emplace_back(8, ProductionShimPhase::before);
  return events;
}

}  // namespace

bool production_capture_dispatcher_selftest_v1() noexcept {
  try {
    DispatcherSyntheticImage image;
    const auto run = [&](const std::uint32_t middle_site) {
      DispatcherFixtureSink sink{image.address()};
      ProductionCaptureCoordinator coordinator;
      if (coordinator.preflight(77, image.address(), dispatcher_fixture_sink(sink)) !=
              ProductionCaptureCoordinatorError::ok ||
          coordinator.impl_ == nullptr) {
        return false;
      }
      coordinator.impl_->fixture_mode = true;
      coordinator.impl_->fixture_events = fixture_events(middle_site);
      if (coordinator.install() != ProductionCaptureCoordinatorError::ok) return false;
      ProductionMachineFrame frame{};
      for (const auto& [site, phase] : coordinator.impl_->fixture_events) {
        if (!ProductionCaptureCoordinator::dispatch(
                coordinator.impl_, site, phase, frame)) {
          return false;
        }
      }
      return coordinator.committed() && sink.seals == 1 && sink.aborts == 0 &&
             coordinator.uninstall() == ProductionCaptureCoordinatorError::ok &&
             coordinator.prepare_unload() == ProductionCaptureCoordinatorError::ok;
    };
    if (!image.ready() || !run(7) || !run(6)) {
      return false;
    }
    const auto rejects = [&](std::vector<std::pair<std::uint32_t, ProductionShimPhase>> events,
                             const bool binding_drift = false) {
      DispatcherFixtureSink sink{image.address()};
      ProductionCaptureCoordinator rejected;
      if (rejected.preflight(77, image.address(), dispatcher_fixture_sink(sink)) !=
              ProductionCaptureCoordinatorError::ok ||
          rejected.impl_ == nullptr) {
        return false;
      }
      rejected.impl_->fixture_mode = true;
      rejected.impl_->fixture_binding_drift = binding_drift;
      rejected.impl_->fixture_events = events;
      if (rejected.install() != ProductionCaptureCoordinatorError::ok) return false;
      ProductionMachineFrame frame{};
      bool refused = false;
      for (const auto& [site, phase] : events) {
        if (!ProductionCaptureCoordinator::dispatch(
                rejected.impl_, site, phase, frame)) {
          refused = true;
          break;
        }
      }
      return refused && rejected.terminal() && sink.seals == 0 && sink.aborts == 1 &&
             rejected.uninstall() == ProductionCaptureCoordinatorError::ok;
    };

    auto missing = fixture_events(7);
    missing.erase(
        std::remove_if(missing.begin(), missing.end(), [](const auto& event) {
          return event.first == 23;
        }),
        missing.end());
    auto reversed = fixture_events(7);
    const auto process_before = std::find(
        reversed.begin(), reversed.end(), std::pair{23u, ProductionShimPhase::before});
    const auto post_after = std::find(
        reversed.begin(), reversed.end(), std::pair{24u, ProductionShimPhase::after});
    if (process_before == reversed.end() || post_after == reversed.end()) return false;
    std::rotate(process_before, process_before + 2, post_after + 1);
    auto duplicate = fixture_events(7);
    const auto duplicate_at = std::find(
        duplicate.begin(), duplicate.end(), std::pair{24u, ProductionShimPhase::before});
    duplicate.insert(
        duplicate_at,
        {{23u, ProductionShimPhase::before}, {23u, ProductionShimPhase::after}});
    auto after_without_before = fixture_events(7);
    const auto orphan_before = std::find(
        after_without_before.begin(),
        after_without_before.end(),
        std::pair{23u, ProductionShimPhase::before});
    after_without_before.erase(orphan_before);
    auto class_after_without_before = fixture_events(7);
    const auto orphan_class_before = std::find(
        class_after_without_before.begin(), class_after_without_before.end(),
        std::pair{25u, ProductionShimPhase::before});
    class_after_without_before.erase(orphan_class_before);

    if (!rejects(fixture_events(7), true) || !rejects(std::move(missing)) ||
        !rejects(std::move(reversed)) || !rejects(std::move(duplicate)) ||
        !rejects(std::move(after_without_before)) ||
        !rejects(std::move(class_after_without_before))) {
      return false;
    }

    DispatcherFixtureSink retry_sink{image.address()};
    retry_sink.abort_failures = 2;
    ProductionCaptureCoordinator retry_abort;
    if (retry_abort.preflight(
            77, image.address(), dispatcher_fixture_sink(retry_sink)) !=
            ProductionCaptureCoordinatorError::ok ||
        retry_abort.impl_ == nullptr) {
      return false;
    }
    retry_abort.impl_->fixture_mode = true;
    retry_abort.impl_->fixture_events = fixture_events(7);
    if (retry_abort.install() != ProductionCaptureCoordinatorError::ok) return false;
    ProductionMachineFrame frame{};
    if (ProductionCaptureCoordinator::dispatch(
            retry_abort.impl_, 1, ProductionShimPhase::before, frame) ||
        retry_sink.aborts != 1 || !retry_abort.recovery_required() ||
        retry_abort.uninstall() != ProductionCaptureCoordinatorError::terminal_failure ||
        retry_sink.aborts != 2 || !retry_abort.recovery_required() ||
        retry_abort.uninstall() != ProductionCaptureCoordinatorError::ok ||
        retry_sink.aborts != 3 || retry_abort.recovery_required()) {
      return false;
    }
    return true;
  } catch (...) {
    return false;
  }
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
