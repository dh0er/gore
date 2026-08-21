# Authorized bridge contract for BuildID 24539464

This is a capture specification, not executable hook code. The static library does not install
these observation points. A later run needs separate authority to start the game and attach or
load an instrumentation bridge.

The bridge must preserve this order:

1. Call `open_pinned` before recording anything. Supply the running primary-image base, the
   exact on-disk EXE path, observed Steam BuildID `24539464`, an output path outside the game
   directory, and a nonzero random capture ID. The helper independently verifies EXE size,
   SHA-256, PE image size, and RSDS GUID/age.
2. At the pinned `SetEngineProperty` implementation (RVA `0x47a50f0`), record each property ID
   and value in call order. The raw engine pointer is not captured.
3. Around each indirect bind callback call at RVA `0x46856fb` / return RVA `0x46856fd`, intern
   the callback address, record begin/end registry counts and a deterministic registry digest,
   and emit every typed `RegistrationEntryV1 + PostBindResultV1` delta. Emit direct post-bind
   state mutations while that callback is active.
4. Once all bind callbacks have returned, materialize and emit exactly one
   `RegistrySupportCaptureV1`: complete `HostStubDescriptorV1` traits, each stub's pointer token,
   all eleven primitive operation descriptors, and both dynamic script operation descriptors.
   Raw addresses, vtables, object bytes, and process-relative VAs are forbidden.
5. Emit one final `PostBindStateV1` for every registered object type, object property, function,
   and global property. The offline decoder requires exact 1:1 trace coverage.
6. At `GetBuildIdentifier` RVA `0x48d3230` and `GetStaticJitInfo` RVA `0x48d0f60`, record the
   returned build/JIT facts, including the Shipping cache GUID comparison and whether the JIT
   database was cleared. The helper accepts only the pinned expected identity.
7. Serialize the three complete frontend JSON configs. Use their canonical digests to compute
   `SHA256("gore-as-captured-frontend-config-set-v1\\0" || preprocessor_digest ||
   class_generator_digest || compiler_options_digest)` and use that digest at every boundary.
8. Record exactly three frontend boundaries: `InitialCompile` entry RVA `0x4684210`; either the
   precompiled-descriptor request RVA `0x46842d0` or preprocessor-constructed RVA `0x468435d`;
   and successful return RVA `0x4685a46`. Source/module inputs and compiler outputs are digests,
   not pointers.
9. Call `seal`. Preserve an unsealed file for diagnosis if a fail-closed check trips; the offline
   decoder will not accept it as profile evidence.

Only the authorized runtime run can supply the effective property values, callback order,
registry deltas/results, host-stub traits, post-bind mutations/final states, current JIT facts,
frontend configuration, and boundary input/output digests. Static analysis established the
target identity and observation RVAs but cannot honestly substitute values for those facts.
