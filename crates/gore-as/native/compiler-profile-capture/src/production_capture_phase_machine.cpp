#define GORE_AS_CAPTURE_BRIDGE_BUILD
#include "production_capture_phase_machine.hpp"

#include "bridge_internal.hpp"
#include "gore_as_capture/hook_table.hpp"
#include "gore_as_capture/live_bootstrap.h"
#include "live_bootstrap_internal.hpp"

#include <Windows.h>

#include <algorithm>
#include <cstring>
#include <limits>
#include <utility>

namespace gore_as_capture::v1::instrumentation {
namespace {

constexpr std::size_t kMaximumBufferedJsonBytes = 256u * 1024u * 1024u;

gore_as_capture_registry_counts_v1 bridge_counts(const RegistryCounts& counts) noexcept {
  return {counts.types, counts.functions, counts.object_properties,
          counts.global_properties, counts.enum_values, counts.funcdefs,
          counts.typedefs, counts.total_registrations};
}

bool bridge_validate(
    void*, const std::uint64_t session_id, const std::uintptr_t image) noexcept {
  gore_as_capture_bridge_contract_v1 contract{};
  return bridge_validate_live_session_v1(session_id, image) &&
         gore_as_capture_bridge_query_v1(&contract) == GORE_AS_CAPTURE_BRIDGE_OK_V1 &&
         contract.abi_version == GORE_AS_CAPTURE_BRIDGE_ABI_V1 &&
         contract.hook_table_version == kHookTableVersion &&
         contract.hook_table_fingerprint == kPinnedHookTableFingerprint &&
         contract.pe_size_of_image == kPeSizeOfImage;
}

std::uint32_t bridge_intern(
    void*, const std::uint64_t session_id, const std::uintptr_t pointer,
    std::uint32_t& token) noexcept {
  return gore_as_capture_bridge_intern_primary_image_pointer_v1(
      session_id, pointer, &token);
}

std::uint32_t bridge_property(
    void*, const std::uint64_t session_id, const std::uint32_t id,
    const std::uint64_t value) noexcept {
  return gore_as_capture_bridge_append_engine_property_v1(
      session_id, id, value, kRvaSetEngineProperty);
}

std::uint32_t bridge_bind_begin(
    void*, const std::uint64_t session_id, const std::uint32_t ordinal,
    const std::int32_t order, const std::uint32_t token,
    const PublicRegistrySnapshot& snapshot) noexcept {
  const auto counts = bridge_counts(snapshot.counts);
  return gore_as_capture_bridge_append_bind_begin_v1(
      session_id, ordinal, order, token, &counts,
      reinterpret_cast<const std::uint8_t*>(snapshot.canonical_sha256.data()));
}

std::uint32_t bridge_bind_end(
    void*, const std::uint64_t session_id, const std::uint32_t ordinal,
    const std::int32_t order, const std::uint32_t token,
    const PublicRegistrySnapshot& snapshot) noexcept {
  const auto counts = bridge_counts(snapshot.counts);
  return gore_as_capture_bridge_append_bind_end_v1(
      session_id, ordinal, order, token, &counts,
      reinterpret_cast<const std::uint8_t*>(snapshot.canonical_sha256.data()));
}

std::uint32_t bridge_json(
    void*, const std::uint64_t session_id, const std::uint32_t kind,
    const std::string& json) noexcept {
  const auto* bytes = reinterpret_cast<const std::uint8_t*>(json.data());
  const auto size = static_cast<std::uint32_t>(json.size());
  switch (kind) {
    case 1:
      return gore_as_capture_bridge_append_registry_delta_json_v1(
          session_id, bytes, size);
    case 2:
      return gore_as_capture_bridge_append_registry_support_json_v1(
          session_id, bytes, size);
    case 3:
      return gore_as_capture_bridge_append_final_post_bind_state_json_v1(
          session_id, bytes, size);
    default:
      return GORE_AS_CAPTURE_BRIDGE_INVALID_ARGUMENT_V1;
  }
}

std::uint32_t bridge_build(
    void*, const std::uint64_t session_id,
    const gore_as_capture_build_jit_v1& fact) noexcept {
  return gore_as_capture_bridge_append_build_jit_v1(session_id, &fact);
}

std::uint32_t bridge_frontend_config(
    void*, const std::uint64_t session_id, const std::uint32_t kind,
    const std::string& json) noexcept {
  return gore_as_capture_bridge_append_frontend_config_json_v1(
      session_id, kind, reinterpret_cast<const std::uint8_t*>(json.data()),
      static_cast<std::uint32_t>(json.size()));
}

std::uint32_t bridge_frontend_boundary(
    void*, const std::uint64_t session_id,
    const FrontendBoundaryProjection& projection) noexcept {
  gore_as_capture_frontend_boundary_v1 value{};
  value.struct_size = sizeof(value);
  value.kind = static_cast<std::uint32_t>(projection.kind);
  value.observation_rva = projection.observation_rva;
  value.module_count = projection.module_count;
  value.result_code = projection.result_code;
  std::memcpy(value.config_sha256, projection.config_sha256.data(),
              projection.config_sha256.size());
  std::memcpy(value.input_sha256, projection.input_sha256.data(),
              projection.input_sha256.size());
  std::memcpy(value.output_sha256, projection.output_sha256.data(),
              projection.output_sha256.size());
  return gore_as_capture_bridge_append_frontend_boundary_v1(session_id, &value);
}

std::uint32_t bridge_abort(void*, const std::uint64_t session_id) noexcept {
  const auto status = gore_as_capture_bridge_abort_and_detach_v1(session_id);
  live_capture_note_outcome_v1(
      status == GORE_AS_CAPTURE_BRIDGE_OK_V1
          ? GORE_AS_CAPTURE_LIVE_OUTCOME_ABORTED_V1
          : GORE_AS_CAPTURE_LIVE_OUTCOME_ABORT_FAILED_V1);
  return status;
}

std::uint32_t bridge_seal(void*, const std::uint64_t session_id) noexcept {
  const auto status = gore_as_capture_bridge_seal_and_detach_v1(session_id);
  live_capture_note_outcome_v1(
      status == GORE_AS_CAPTURE_BRIDGE_OK_V1
          ? GORE_AS_CAPTURE_LIVE_OUTCOME_SEALED_V1
          : GORE_AS_CAPTURE_LIVE_OUTCOME_SEAL_FAILED_V1);
  return status;
}

bool nonzero_digest(const Digest& digest) noexcept {
  return std::any_of(digest.begin(), digest.end(), [](const auto byte) {
    return byte != std::byte{0};
  });
}

bool valid_json_buffer(const std::string& value) noexcept {
  return value.size() >= 2 && value.size() <= kMaximumBufferedJsonBytes &&
         value.front() == '{' && value.back() == '}' &&
         value.find('\0') == std::string::npos;
}

}  // namespace

ProductionCaptureSink production_bridge_sink_v1() noexcept {
  return {nullptr, bridge_validate, bridge_intern, bridge_property,
          bridge_bind_begin, bridge_bind_end, bridge_json, bridge_build,
          bridge_frontend_config, bridge_frontend_boundary, bridge_seal, bridge_abort};
}

bool ProductionCapturePhaseMachine::valid_owner() const noexcept {
  return owner_thread_ != 0 && owner_thread_ == GetCurrentThreadId();
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::reject(
    const ProductionCapturePhaseError error) noexcept {
  terminal_ = true;
  if (preflighted_ && !committed_ && !abort_complete_ && valid_owner() &&
      sink_.abort != nullptr) {
    abort_complete_ =
        sink_.abort(sink_.context, session_id_) == GORE_AS_CAPTURE_BRIDGE_OK_V1;
  }
  return error;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::sink_failure() noexcept {
  if (!terminal_ && sink_.abort != nullptr) {
    abort_complete_ =
        sink_.abort(sink_.context, session_id_) == GORE_AS_CAPTURE_BRIDGE_OK_V1;
  }
  terminal_ = true;
  return ProductionCapturePhaseError::sink_rejected;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::preflight(
    const std::uint64_t session_id, const std::uintptr_t primary_image,
    const ProductionCaptureSink sink) noexcept {
  if (preflighted_ || terminal_ || session_id == 0 || primary_image == 0 ||
      sink.validate == nullptr || sink.intern_pointer == nullptr ||
      sink.engine_property == nullptr || sink.bind_begin == nullptr ||
      sink.bind_end == nullptr || sink.json == nullptr || sink.build_jit == nullptr ||
      sink.frontend_config == nullptr || sink.frontend_boundary == nullptr ||
      sink.seal == nullptr || sink.abort == nullptr) {
    return reject(ProductionCapturePhaseError::invalid_argument);
  }
  if (!sink.validate(sink.context, session_id, primary_image)) {
    return reject(ProductionCapturePhaseError::target_drift);
  }
  session_id_ = session_id;
  primary_image_ = primary_image;
  owner_thread_ = GetCurrentThreadId();
  sink_ = sink;
  preflighted_ = true;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::adopt_runtime_owner() noexcept {
  if (!preflighted_ || terminal_ || bind_active_ || registry_complete_ || build_complete_ ||
      frontend_complete_ || !properties_.empty() || !pointer_capabilities_.empty() ||
      !binds_.empty() || !support_json_.empty() || !final_state_json_.empty() ||
      !boundaries_.empty()) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  owner_thread_ = GetCurrentThreadId();
  return owner_thread_ != 0 ? ProductionCapturePhaseError::ok
                            : reject(ProductionCapturePhaseError::wrong_thread);
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::add_engine_property(
    const std::uint32_t property_id, const std::uint64_t value) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  // SetEngineProperty is an ordered call stream. The target performs a small
  // bootstrap registration prefix before finishing this setup stream, while
  // the sealed replay format intentionally applies all properties first.
  // Preserve property order, but allow that capture-time interleaving until
  // the registry has reached its final boundary.
  if (registry_complete_ || property_id == 0 || property_id > 34) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  try {
    properties_.push_back({property_id, value});
    return ProductionCapturePhaseError::ok;
  } catch (...) {
    return reject(ProductionCapturePhaseError::limit_exceeded);
  }
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::intern_primary_image_pointer(
    const std::uintptr_t pointer, std::uint32_t& token) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (registry_complete_ || pointer <= primary_image_ ||
      pointer - primary_image_ >= kPeSizeOfImage) {
    return reject(ProductionCapturePhaseError::invalid_argument);
  }
  const auto found = std::find(pointer_capabilities_.begin(), pointer_capabilities_.end(),
                               pointer);
  if (found != pointer_capabilities_.end()) {
    token = static_cast<std::uint32_t>(found - pointer_capabilities_.begin());
    return ProductionCapturePhaseError::ok;
  }
  if (pointer_capabilities_.size() >= std::numeric_limits<std::uint32_t>::max()) {
    return reject(ProductionCapturePhaseError::limit_exceeded);
  }
  try {
    token = static_cast<std::uint32_t>(pointer_capabilities_.size());
    pointer_capabilities_.push_back(pointer);
    return ProductionCapturePhaseError::ok;
  } catch (...) {
    return reject(ProductionCapturePhaseError::limit_exceeded);
  }
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::begin_bind(
    const std::int32_t bind_order, const std::uint32_t callback_pointer_token,
    const PublicRegistrySnapshot& baseline) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (bind_active_ || registry_complete_ ||
      callback_pointer_token >= pointer_capabilities_.size() ||
      !nonzero_digest(baseline.canonical_sha256)) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  try {
    binds_.push_back({static_cast<std::uint32_t>(binds_.size()), bind_order,
                      callback_pointer_token, baseline, {}, {}});
    bind_active_ = true;
    return ProductionCapturePhaseError::ok;
  } catch (...) {
    return reject(ProductionCapturePhaseError::limit_exceeded);
  }
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::add_registry_delta(
    std::string json) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (!bind_active_ || binds_.empty() || !valid_json_buffer(json)) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  try {
    binds_.back().deltas.push_back(std::move(json));
    return ProductionCapturePhaseError::ok;
  } catch (...) {
    return reject(ProductionCapturePhaseError::limit_exceeded);
  }
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::end_bind(
    const PublicRegistrySnapshot& final_snapshot) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (!bind_active_ || binds_.empty() ||
      !nonzero_digest(final_snapshot.canonical_sha256)) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  binds_.back().final_snapshot = final_snapshot;
  bind_active_ = false;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::replace_registry_deltas(
    std::vector<std::vector<std::string>> deltas) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (bind_active_ || registry_complete_ || deltas.size() != binds_.size()) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  for (std::size_t bind = 0; bind < deltas.size(); ++bind) {
    if (deltas[bind].size() != binds_[bind].deltas.size() ||
        std::any_of(deltas[bind].begin(), deltas[bind].end(),
                    [](const auto& json) { return !valid_json_buffer(json); })) {
      return reject(ProductionCapturePhaseError::invalid_order);
    }
  }
  for (std::size_t bind = 0; bind < deltas.size(); ++bind) {
    binds_[bind].deltas = std::move(deltas[bind]);
  }
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::complete_registry(
    std::string support_json, std::vector<std::string> final_state_json) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (bind_active_ || binds_.empty() || registry_complete_ ||
      !valid_json_buffer(support_json) || final_state_json.empty() ||
      std::any_of(final_state_json.begin(), final_state_json.end(),
                  [](const auto& json) { return !valid_json_buffer(json); })) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  support_json_ = std::move(support_json);
  final_state_json_ = std::move(final_state_json);
  registry_complete_ = true;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::set_build_jit(
    const gore_as_capture_build_jit_v1& fact) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (!registry_complete_ || build_complete_ || fact.struct_size != sizeof(fact) ||
      fact.build_identifier != kBuildIdentifier || fact.shipping_cache_matches != 1 ||
      fact.as_reference_debugging != 0 ||
      fact.fork_opcode_table_201_212_present != 1 ||
      fact.reference_debug_opcodes_emittable != 0 ||
      fact.resolve_object_ptr_callback_registered != 0) {
    return reject(ProductionCapturePhaseError::target_drift);
  }
  build_jit_ = fact;
  build_complete_ = true;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::set_frontend(
    std::string preprocessor_json, std::string class_generator_json,
    std::string compiler_options_json,
    std::vector<FrontendBoundaryProjection> boundaries) noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (!registry_complete_ || frontend_complete_ ||
      !valid_json_buffer(preprocessor_json) ||
      !valid_json_buffer(class_generator_json) ||
      !valid_json_buffer(compiler_options_json) || boundaries.size() != 3 ||
      boundaries[0].kind != FrontendBoundaryKind::initial_compile_enter ||
      (boundaries[1].kind != FrontendBoundaryKind::precompiled_descriptors_requested &&
       boundaries[1].kind != FrontendBoundaryKind::preprocessor_constructed) ||
      boundaries[2].kind != FrontendBoundaryKind::initial_compile_return) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  frontend_json_[0] = std::move(preprocessor_json);
  frontend_json_[1] = std::move(class_generator_json);
  frontend_json_[2] = std::move(compiler_options_json);
  boundaries_ = std::move(boundaries);
  frontend_complete_ = true;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::complete() noexcept {
  if (!preflighted_ || terminal_) return ProductionCapturePhaseError::terminal_failure;
  if (!valid_owner()) return reject(ProductionCapturePhaseError::wrong_thread);
  if (committed_ || bind_active_ || properties_.empty() || !registry_complete_ ||
      !build_complete_ || !frontend_complete_) {
    return reject(ProductionCapturePhaseError::invalid_order);
  }
  for (const auto& property : properties_) {
    if (sink_.engine_property(sink_.context, session_id_, property.id, property.value) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  }
  for (std::size_t index = 0; index < pointer_capabilities_.size(); ++index) {
    std::uint32_t token = 0;
    if (sink_.intern_pointer(sink_.context, session_id_, pointer_capabilities_[index],
                            token) != GORE_AS_CAPTURE_BRIDGE_OK_V1 || token != index) {
      return sink_failure();
    }
  }
  for (const auto& bind : binds_) {
    if (sink_.bind_begin(sink_.context, session_id_, bind.ordinal, bind.order,
                         bind.callback_token, bind.baseline) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
    for (const auto& delta : bind.deltas) {
      if (sink_.json(sink_.context, session_id_, 1, delta) !=
          GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
    }
    if (sink_.bind_end(sink_.context, session_id_, bind.ordinal, bind.order,
                       bind.callback_token, bind.final_snapshot) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  }
  if (sink_.json(sink_.context, session_id_, 2, support_json_) !=
      GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  for (const auto& json : final_state_json_) {
    if (sink_.json(sink_.context, session_id_, 3, json) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  }
  if (sink_.build_jit(sink_.context, session_id_, build_jit_) !=
      GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  for (std::size_t index = 0; index < frontend_json_.size(); ++index) {
    if (sink_.frontend_config(sink_.context, session_id_,
                              static_cast<std::uint32_t>(index + 1),
                              frontend_json_[index]) != GORE_AS_CAPTURE_BRIDGE_OK_V1) {
      return sink_failure();
    }
  }
  for (const auto& boundary : boundaries_) {
    if (sink_.frontend_boundary(sink_.context, session_id_, boundary) !=
        GORE_AS_CAPTURE_BRIDGE_OK_V1) return sink_failure();
  }
  if (sink_.seal(sink_.context, session_id_) != GORE_AS_CAPTURE_BRIDGE_OK_V1) {
    return sink_failure();
  }
  committed_ = true;
  terminal_ = true;
  return ProductionCapturePhaseError::ok;
}

ProductionCapturePhaseError ProductionCapturePhaseMachine::abort() noexcept {
  if (!preflighted_ || committed_ || abort_complete_) {
    return ProductionCapturePhaseError::terminal_failure;
  }
  if (!valid_owner()) {
    terminal_ = true;
    return ProductionCapturePhaseError::wrong_thread;
  }
  const auto status = sink_.abort(sink_.context, session_id_);
  terminal_ = true;
  abort_complete_ = status == GORE_AS_CAPTURE_BRIDGE_OK_V1;
  return abort_complete_ ? ProductionCapturePhaseError::ok
                         : ProductionCapturePhaseError::sink_rejected;
}

#if defined(GORE_AS_CAPTURE_TEST_TARGET)
namespace {
struct FixtureSink final {
  std::vector<std::uint32_t> order;
  std::uint32_t fail_at{std::numeric_limits<std::uint32_t>::max()};
  std::uint32_t calls{};
  std::uint32_t aborts{};
};

std::uint32_t fixture_append(FixtureSink& sink, const std::uint32_t kind) noexcept {
  if (sink.calls++ == sink.fail_at) return GORE_AS_CAPTURE_BRIDGE_IO_ERROR_V1;
  sink.order.push_back(kind);
  return GORE_AS_CAPTURE_BRIDGE_OK_V1;
}

ProductionCaptureSink fixture_sink(FixtureSink& sink) noexcept {
  return {
      &sink,
      [](void*, std::uint64_t session, std::uintptr_t image) noexcept {
        return session == 7 && image == 0x140000000ull;
      },
      [](void* context, std::uint64_t, std::uintptr_t, std::uint32_t& token) noexcept {
        auto& value = *static_cast<FixtureSink*>(context);
        token = static_cast<std::uint32_t>(
            std::count(value.order.begin(), value.order.end(), 2u));
        return fixture_append(value, 2);
      },
      [](void* context, std::uint64_t, std::uint32_t, std::uint64_t) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 1);
      },
      [](void* context, std::uint64_t, std::uint32_t, std::int32_t,
         std::uint32_t, const PublicRegistrySnapshot&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 3);
      },
      [](void* context, std::uint64_t, std::uint32_t, std::int32_t,
         std::uint32_t, const PublicRegistrySnapshot&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 5);
      },
      [](void* context, std::uint64_t, std::uint32_t kind,
         const std::string&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 5 + kind);
      },
      [](void* context, std::uint64_t, const gore_as_capture_build_jit_v1&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 9);
      },
      [](void* context, std::uint64_t, std::uint32_t kind,
         const std::string&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 9 + kind);
      },
      [](void* context, std::uint64_t, const FrontendBoundaryProjection&) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 13);
      },
      [](void* context, std::uint64_t) noexcept {
        return fixture_append(*static_cast<FixtureSink*>(context), 14);
      },
      [](void* context, std::uint64_t) noexcept -> std::uint32_t {
        auto& value = *static_cast<FixtureSink*>(context);
        ++value.aborts;
        return GORE_AS_CAPTURE_BRIDGE_OK_V1;
      }};
}

bool drive_fixture(
    ProductionCapturePhaseMachine& machine,
    FixtureSink& sink,
    const bool frontend_first = false) {
  PublicRegistrySnapshot snapshot{};
  snapshot.canonical_sha256[0] = std::byte{1};
  std::uint32_t token = 0;
  gore_as_capture_build_jit_v1 build{};
  build.struct_size = sizeof(build);
  build.build_identifier = kBuildIdentifier;
  build.shipping_cache_matches = 1;
  build.fork_opcode_table_201_212_present = 1;
  FrontendDigest digest{};
  digest[0] = 1;
  std::vector<FrontendBoundaryProjection> boundaries{
      {FrontendBoundaryKind::initial_compile_enter, kRvaInitialCompileEnter, 0, 0,
       digest, {}, {}},
      {FrontendBoundaryKind::preprocessor_constructed,
       kRvaPreprocessorConstructed, 0, 0, digest, {}, {}},
      {FrontendBoundaryKind::initial_compile_return, kRvaInitialCompileReturn, 1, 0,
       digest, {}, digest}};
  if (machine.preflight(7, 0x140000000ull, fixture_sink(sink)) !=
      ProductionCapturePhaseError::ok) {
    return false;
  }
  if (machine.add_engine_property(1, 2) != ProductionCapturePhaseError::ok ||
      machine.intern_primary_image_pointer(0x140001000ull, token) !=
          ProductionCapturePhaseError::ok ||
      token != 0 ||
      machine.begin_bind(3, token, snapshot) != ProductionCapturePhaseError::ok ||
      machine.add_registry_delta("{}") != ProductionCapturePhaseError::ok ||
      machine.end_bind(snapshot) != ProductionCapturePhaseError::ok ||
      machine.replace_registry_deltas({{"{\"replacement\":true}"}}) !=
          ProductionCapturePhaseError::ok ||
      machine.complete_registry("{}", {"{}"}) != ProductionCapturePhaseError::ok) {
    return false;
  }
  if (frontend_first) {
    return machine.set_frontend("{}", "{}", "{}", std::move(boundaries)) ==
               ProductionCapturePhaseError::ok &&
           machine.set_build_jit(build) == ProductionCapturePhaseError::ok;
  }
  return machine.set_build_jit(build) == ProductionCapturePhaseError::ok &&
         machine.set_frontend("{}", "{}", "{}", std::move(boundaries)) ==
             ProductionCapturePhaseError::ok;
}
}  // namespace

bool production_capture_phase_machine_selftest_v1() noexcept {
  try {
    FixtureSink sink;
    ProductionCapturePhaseMachine machine;
    if (!drive_fixture(machine, sink) ||
        machine.complete() != ProductionCapturePhaseError::ok || !machine.committed() ||
        sink.aborts != 0 || machine.abort() != ProductionCapturePhaseError::terminal_failure ||
        sink.aborts != 0 ||
        sink.order != std::vector<std::uint32_t>{1, 2, 3, 6, 5, 7, 8, 9,
                                                 10, 11, 12, 13, 13, 13, 14}) {
      return false;
    }
    FixtureSink frontend_first_sink;
    ProductionCapturePhaseMachine frontend_first;
    if (!drive_fixture(frontend_first, frontend_first_sink, true) ||
        frontend_first.complete() != ProductionCapturePhaseError::ok ||
        !frontend_first.committed() || frontend_first_sink.aborts != 0 ||
        frontend_first_sink.order != sink.order) {
      return false;
    }
    FixtureSink failed;
    failed.fail_at = 3;
    ProductionCapturePhaseMachine rejected;
    if (!(drive_fixture(rejected, failed) &&
           rejected.complete() == ProductionCapturePhaseError::sink_rejected &&
           rejected.terminal() && failed.aborts == 1)) {
      return false;
    }
    FixtureSink semantic;
    ProductionCapturePhaseMachine invalid;
    PublicRegistrySnapshot snapshot{};
    snapshot.canonical_sha256[0] = std::byte{1};
    std::uint32_t token = 0;
    return invalid.preflight(7, 0x140000000ull, fixture_sink(semantic)) ==
               ProductionCapturePhaseError::ok &&
           invalid.add_engine_property(1, 2) == ProductionCapturePhaseError::ok &&
           invalid.intern_primary_image_pointer(0x140001000ull, token) ==
               ProductionCapturePhaseError::ok &&
           invalid.begin_bind(3, token, snapshot) == ProductionCapturePhaseError::ok &&
           invalid.add_engine_property(2, 3) == ProductionCapturePhaseError::ok &&
           invalid.add_registry_delta("{}") == ProductionCapturePhaseError::ok &&
           invalid.end_bind(snapshot) == ProductionCapturePhaseError::ok &&
           invalid.complete_registry("{}", {"{}"}) == ProductionCapturePhaseError::ok &&
           invalid.add_engine_property(3, 4) == ProductionCapturePhaseError::invalid_order &&
           invalid.terminal() && !invalid.needs_abort() && semantic.aborts == 1 &&
           invalid.abort() == ProductionCapturePhaseError::terminal_failure &&
           semantic.aborts == 1;
  } catch (...) {
    return false;
  }
}
#endif

}  // namespace gore_as_capture::v1::instrumentation
