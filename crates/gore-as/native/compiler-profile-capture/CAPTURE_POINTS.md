# Authorized bridge contract for BuildID 24878692

This is the runtime observation specification. The buildable production bridge DLL exposes the
exact BuildID-24878692/RVA table, attach/record/seal/detach ABI, and target-specific instrumentation
contract documented in `INSTRUMENTATION.md`. It never installs anything from `DllMain`, starts a
process, or attaches to one. Static analysis closed every exact transfer/frame ABI, a separate
exact fourteen-entry registration-hook contract, the public AS 2.33 registry projection, bounded
target-witnessed FinalState field extractors, canonical Delta/HostStub/FinalState serializers and
ID correlation, the target TypeUsage helper/operation vtable, Build/JIT/compiler-flag extraction,
frontend boundary frames, preprocessor module/source/hash layout, three transient frontend callback
callsites, pointer-neutral frontend semantic projections and the transactional patch mechanics.
The bounded immutable-snapshot frontend materializer now closes the target-raw structures. Static
target-xref proof also closes both graph hooks as unbound: their exact 24-byte runtime objects must
remain `{pointer=0,num=0,max=0,compaction=2,broadcast_count=0}`, and the resulting frontend config
must carry ProcessChunks/PostProcessCode `bound=false` with empty captures. Drift is terminal before
recording. The bounded sparse CurrentProcess snapshot builder, direct 26-site dispatcher and one
RawMaterializer -> registration/frontend -> serializer -> Bridge phase machine are now connected.
The production contract reports `production_installable=1`; the separately named synthetic
fixture reports `0`. BuildID 24539464 remains a typed historical offline decoder/materializer
target, but the live bridge never selects it.

The nine observation RVAs are independently pinned per generation:

| Observation | 24539464 historical | 24878692 live |
| --- | ---: | ---: |
| SetEngineProperty | `0x47a50f0` | `0x47a50b0` |
| bind call / return | `0x46856fb` / `0x46856fd` | `0x46856bb` / `0x46856bd` |
| GetBuildIdentifier | `0x48d3230` | `0x48d31f0` |
| GetStaticJitInfo | `0x48d0f60` | `0x48d0f20` |
| InitialCompile entry | `0x4684210` | `0x46841d0` |
| descriptors requested | `0x46842d0` | `0x4684290` |
| preprocessor constructed | `0x468435d` | `0x468431d` |
| InitialCompile return | `0x4685a46` | `0x4685a06` |

The bridge must preserve this order:

1. Call `open_pinned` before recording anything. Supply the running primary-image base, the
   exact on-disk EXE path, observed Steam BuildID `24878692`, an output path outside the resolved
   executable directory, and a nonzero random capture ID. The helper independently verifies EXE
   size, SHA-256, PE image size, RSDS GUID/age, source/loaded-module handle identity, and the
   handle-resolved output location. A redirected output that reaches the executable tree is
   removed only through its newly created handle; cleanup failure is recovery-required.
2. At the pinned `SetEngineProperty` implementation (RVA `0x47a50b0`), record each property ID
   and value in call order. The raw engine pointer is not captured.
3. Around each indirect bind callback call at RVA `0x46856bb` / return RVA `0x46856bd`, intern
   the callback address, record begin/end registry counts and a deterministic registry digest,
   and emit every typed `RegistrationEntryV1 + PostBindResultV1` delta. Emit direct post-bind
   state mutations while that callback is active.
4. Once all bind callbacks have returned, materialize and emit exactly one
   `RegistrySupportCaptureV1`: complete `HostStubDescriptorV1` traits, each stub's pointer token,
   all eleven primitive operation descriptors, and both dynamic script operation descriptors.
   Raw addresses, vtables, object bytes, and process-relative VAs are forbidden.
5. Emit one final `PostBindStateV1` for every registered object type, object property, function,
   and global property. The offline decoder requires exact 1:1 trace coverage.
6. At `GetBuildIdentifier` RVA `0x48d31f0` and `GetStaticJitInfo` RVA `0x48d0f20`, record the
   returned build/JIT facts, including the Shipping cache GUID comparison and whether the JIT
   database was cleared. The flags field is exactly: bits 0..3 `jit_info_present`,
   `jit_guid_matches`, `jit_database_cleared`, `shipping_cache_matches`; bit 4
   `as_reference_debugging`; bit 5 `fork_opcode_table_201_212_present`; bit 6
   `reference_debug_opcodes_emittable`; bit 7 `resolve_object_ptr_callback_registered`. All higher
   bits are reserved. BuildID 24878692 requires bits 4..7 to equal `0x20` (false, true, false,
   false). Opcode-table presence is not opcode reachability: qualification may expect opcode 204
   only when bit 7 is true and opcodes 206..208 only when bit 6 is true. The helper accepts only
   the pinned expected identity.
7. Serialize the three complete frontend JSON configs. Use their canonical digests to compute
   `SHA256("gore-as-captured-frontend-config-set-v1\\0" || preprocessor_digest ||
   class_generator_digest || compiler_options_digest)` and use that digest at every boundary.
8. Record exactly three frontend boundaries: `InitialCompile` entry RVA `0x46841d0`; either the
   precompiled-descriptor request RVA `0x4684290` or preprocessor-constructed RVA `0x468431d`;
   and successful return RVA `0x4685a06`. Source/module inputs and compiler outputs are digests,
   not pointers.
9. The coordinator calls `seal_and_detach` only after all canonical appends succeed. A fail-closed
   check calls `abort_and_detach`; the retained unsealed file is diagnostic-only and the offline
   decoder will not accept it as profile evidence.

Only the authorized runtime run can supply the effective property calls; callback and registration
sequence/results; encountered pointer-capability identities; resulting registry/final values;
current JIT presence/GUID/clear outcome; and actual frontend configs, descriptor/module/source
sets and outputs. Static analysis has established the target identity, instruction/frame and
fourteen registration contracts, target context/result semantics and statically decidable compiler
flags, but cannot honestly substitute effective values. Object/global target layouts, TypeUsage
operations, Delta, HostStub and FinalState serialization are now statically closed. The fourteen
frames map transactionally to typed projections with 1:1 post-bind enumeration. Frontend hashing
is closed after the new immutable-snapshot materializer has supplied pointer-neutral graphs. For
both authenticated generations' graph delegates are provably unbound, so no graph mutation schema is needed and
their captures must remain empty. State-preserving production shims and their all-or-nothing patch
coordinator, bounded typed-region snapshot builder and fail-closed semantic dispatcher are
statically closed. What remains is exclusively the separately authorized observation of the
effective values above; runtime authority is not permission to guess them.

The observation RVA `0x468431d` is inside the relative operand of the call instruction beginning
at `0x468431c`. The production adapter pins the entire five-byte call as its patch anchor; a direct
patch at `0x468431d` is forbidden.

The offline synthetic host exercises the same phase machine with a separately named fixture DLL.
Its JSON bodies are deliberately structural fixtures, not runtime measurements and not valid
profile evidence. `gore_as_capture_materializer.exe` produces only a wire-audit summary; the Rust
`decode_capture_v1` function consumes the original sealed `.capture`, and a later package
materializer consumes that decoder's typed projections. No synthetic artifact may be catalogued
as a product profile.
