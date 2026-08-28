# BuildID 24878692 exact instrumentation adapter

This module is the buildable, target-specific boundary between the nine static observation
locations and the existing capture bridge ABI. It was developed entirely offline. No process was
started, opened, injected into, or loaded with the production DLL.

The local read-only analysis input was exactly:

- `G1R-Win64-Shipping.exe`, Steam BuildID `24878692`;
- 171,792,384 bytes;
- SHA-256 `824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0`;
- PE `SizeOfImage=0x0a7e5000`, DLL characteristics `0x8160`, GuardFlags `0x100`;
- RSDS `C2CA4ADA-4878-D963-E567-717DC2C483A2`, age 1.

`format.hpp` and the component target tables also retain the fully typed BuildID-24539464
identity. That historical description is used only for offline decoding/materialization. The
production bridge aliases 24878692. No component derives new addresses from a global delta:
compiler code, vtables, early/later data, unwind data and tail code have different shifts.

`instrumentation.cpp` checks the loaded primary module, AMD64/PE32+, SizeOfImage, loaded RSDS
record and all spans below before considering any patch. The bridge separately checks the exact
on-disk file/loaded-module identity and SHA-256 when its session is opened.

## Exact instruction spans

Observation RVAs remain the nine values exposed by `hook_table.hpp`. A patch anchor may begin
earlier only when the semantic observation is inside a complete instruction which must be
relocated as a unit.

| Observation | Observation RVA | Patch anchor | Exact bytes |
| --- | ---: | ---: | --- |
| SetEngineProperty | `047a50b0` | `047a50b0` | `ff ca 83 fa 21 0f 87 7a 02 00 00 48 63 c2` |
| bind callback call | `046856bb` | `046856b8` | `48 8b c8 ff d7` |
| bind callback return | `046856bd` | `046856bd` | `49 83 c7 50 4d 8d 76 50` |
| GetBuildIdentifier | `048d31f0` | `048d31f0` | `48 89 5c 24 18` |
| GetStaticJitInfo | `048d0f20` | `048d0f20` | `48 8b 05 41 c5 49 05 c3` |
| InitialCompile entry | `046841d0` | `046841d0` | `4c 8b dc 55 53 49 8d ab 98 fe ff ff` |
| precompiled descriptors requested | `04684290` | `04684290` | `e8 bb 03 25 00` |
| preprocessor constructed | `0468431d` | `0468431c` | `e8 9f 14 20 00` |
| InitialCompile successful return | `04685a06` | `04685a06` | `48 8b 8b 68 04 00 00` |

The published `0468431d` preprocessor observation is byte 1 of the relative-call instruction at
`0468431c`, not an instruction boundary. Patching `0468431d` directly would corrupt the call
target. The adapter therefore pins and relocates the whole five-byte instruction while retaining
`0468431d` as the capture record's semantic observation.

The prolog table has a separate FNV-1a drift fingerprint in addition to the existing hook-table
fingerprint. Neither fingerprint is an authenticity root; the catalogued DLL and target seals are.

## Static RE closure achieved

Address-driven disassembly of the pinned executable plus the matching AngelScript 2.33.0 donor
now closes the following facts without executing the game:

- **SetEngineProperty.** The observation precedes `dec edx`; EDX is the original property ID and
  R8 is its pointer-width value. RCX is a raw engine capability and is never serialized.
- **Bind loop.** RBX is the manager, R15 the current 0x50-byte bind record, R12 the end pointer and
  RDI the callback. The callback receives the callable storage returned immediately before the
  call; it does not receive the engine pointer. `[manager+0x28]` is the engine capability and the
  callback order is the signed field at record offset zero. The before/after frames and final
  callback condition are typed and range checked.
- **Central registration calls.** A separate, equally BuildID-pinned extension covers all fourteen
  `asCScriptEngine::Register*` entrypoints (engine vtable RVA `0x081f5078`, slots 10, 14, 17..22,
  25, 27, 29, 30, 33 and 36; function RVAs `0x4793870`, `0x4793f90`, `0x4799770`,
  `0x4799250`, `0x4798f10`, `0x4798b90`, `0x4796490`, `0x4796c10`, `0x479d4f0`,
  `0x4791ce0`, `0x47927b0`, `0x4792ad0`, `0x4793380`, `0x479dbe0`). Each contract pins
  its full overwritten prolog, original unwind-info RVA, generated unwind operations, Win64
  register/stack argument sources and EAX result. The typed extractor retains global cross-kind
  order, call convention (including `THISCALL_ASGLOBAL`), auxiliary capability and the fork caller
  descriptor. Return values are projected as type/function IDs, property/enum indices or exact
  installation success. The registration context is also closed: access mask is
  `[engine+0x1558]`, namespace is the bounded `asCString` reached through `[engine+0x1560]`, and
  config groups are absent because vtable slots 39..41 all resolve to the zero-return stub at RVA
  `0x10026c0`.
- **Public registry projection.** `target_snapshot.cpp` implements the exact public AngelScript
  2.33.0 vtable slots and a pointer-neutral SHA-256/count projection for global functions and
  properties, object types and their factories/methods/behaviours/properties, enums, funcdefs,
  typedefs, base/interface type IDs, string-factory type and default-array type. Every invoked
  vtable/function target must be inside the already pinned primary image. Its broad fixture covers
  every registry family, base/interface relations, deterministic replay, semantic drift and
  rejection of an external vtable target.
- **Post-bind field extraction.** `target_final_state.cpp` provides bounded, pointer-neutral typed
  extractors for the complete `PostBindStateV1` field shapes. Exact target instruction witnesses
  close every function field (including hidden argument/default, output-type argument, compile-out
  values 0..3 and first-param metadata), the complete object-property tail and global-property
  pure/storage state. The remaining object-type layout is independently target-closed: allocation
  RVAs `0x479a2d8`, `0x479aa58` and `0x479abba` allocate `0x2d8` bytes and call constructor RVA
  `0x46bc8e0`; that constructor proves alignment `+0x008`, interfaces `+0x190`, interface VFT
  offsets `+0x1b0`, base `+0x1d0`, shadow `+0x1f8` and the four booleans at `+0x2d0..+0x2d3`.
  Object-vtable RVA `0x81f4d90` independently reads base and the interface count/vector through
  those offsets. This corrects the donor-only offsets, which are 0x30 earlier after the interface
  region and are not used for target extraction. All capabilities must resolve to trace IDs,
  arrays are bounded and 1:1, strings are strict UTF-8, and invalid enum/bool values fail closed.
- **Delta, HostStub and FinalState JSON.** `target_capture_serializer.cpp` implements the exact
  schema-v1 projection for all fourteen registration entry/result variants and all four final-state
  variants. It maintains capture trace IDs separately from private engine IDs, correlates owner and
  member indices, preserves callback/registration order, uses first-seen stub IDs, hashes sorted
  pointer-free multi-use callable/object witnesses with explicit NUL-terminated v1 domains, and
  emits storage traits, all eleven pinned primitive operations and both pinned dynamic delegate
  operations. Witnesses are derived transactionally from the typed registration kind, namespace,
  access mask, declaration, call convention, owner trace ID, composite/accessor shape and, where
  applicable, behaviour/template adapter; no caller-supplied generic witness or pointer enters the
  hash. `thiscall_as_global` and auxiliary-object presence are required to agree. Call conventions,
  behaviours and template adapters are closed enums. Final-state emission is a canonical sequence:
  registration order filtered to the four required identity classes, exactly once, with contiguous
  state ordinals; missing, duplicate or out-of-order identities cannot complete. The Rust decoder
  independently repeats 1:1 trace coverage and rejects any target fixed-operation drift. This closes
  the serializers, descriptor derivation and ID machinery.
- **Target type-usage operations.** `target_type_usage.cpp` pins
  `FAngelscriptTypeUsage::FromTypeId` at RVA `0x474d8b0` and its destructor at RVA
  `0x465c090`, including both prologs. The target return object is exactly 0x30 bytes:
  `SubTypes +0x00`, the `TSharedPtr<FAngelscriptType> +0x10`, reference/const bits at
  `+0x20/+0x21`, and the script-class/property/type-index union at `+0x28`. Independent target
  validators at `0x4834e50..0x48353fc` and `0x484dd89` close the type vtable byte offsets for
  object-pointer (`+0x80`), template-subtype (`+0x90`), copy (`+0xa8/+0xb0`), compare (`+0xc0`),
  construct (`+0xd0/+0xd8`), size (`+0xe8`), destruct (`+0xf0/+0xf8`), hash (`+0x148`) and
  alignment (`+0x158`). The BuildID-24878692 ClassGenerator loop at
  `0x485e241`, `0x485e261`, and `0x485e2c9` independently closes
  `NeverRequiresGC` (`+0x70`), `RequiresProperty` (`+0x78`), and
  `CanCreateProperty` (`+0x48`), including the exact struct/default-property
  decision and rejection path. The implementation bounds the recursive subtype array, admits only
  image-internal virtual targets, validates every bool/size/alignment result, destroys the
  temporary with the pinned target destructor and classifies the four closed container heads
  (`TArray`, `TMap`, `TSet`, `TOptional`) from the registered declaration. No target pointer is
  projected.
- **Fourteen-frame semantic observer.** `target_registration_observer.cpp` now consumes the shared
  raw frame representation produced by `instrumentation.cpp` and transactionally maps all fourteen
  entry/result pairs to `RegistrationEntryJsonProjection` and `RegistrationResultJsonProjection`.
  It revalidates the exact hook-kind/argument-semantic contract, captures namespace/access-mask at
  entry, filters object flags to the public `0x003fffff` mask, preserves signed offsets and enum
  values, closes all call-convention/behaviour enums, and maps template callbacks only from the
  nine declaration heads. Container adapters must agree with the recursively validated TypeUsage
  root and arity. Callable identity is the typed caller thunk when present and otherwise the first
  callable word of the validated `asSFuncPtr`; auxiliary presence must agree exactly with
  `THISCALL_ASGLOBAL` before a pointer token is requested.

  Successful EAX results are resolved through public engine slots 49/53 and correlated to stable
  trace IDs. Owner declarations resolve through slot 55 and must name an earlier correlated type.
  Slot 16 (`GetGlobalPropertyByIndex`, RVA `0x4755790`) supplies the effective type ID and storage;
  the returned address must equal both the raw registration argument and the real-address field of
  the exact private property at target engine array `+0x628`. TypeUsage then supplies storage size
  and alignment. Object-property identities use the target-witnessed owner array at `+0x90`.
  Post-bind enumeration re-resolves every retained type/function/property capability, requires the
  full before/after `RegistryCounts` delta to equal the fourteen-kind trace, and feeds the existing
  canonical sequence; missing, extra, reordered or replaced identities cannot seal.
- **Build/JIT.** `GetCurrentBuildIdentifier` returns `0x9e377abe` in EAX. `GetStaticJitInfo` returns
  `[image+0x09d6d468]`. The manager's precompiled object is at `+0x460`, its GUID at `+0x28`, and
  the Initialize CFG compares that GUID with the first 16 bytes of non-null StaticJITInfo and
  reaches `FJITDatabase::Clear` exactly on mismatch.
- **Frontend frames and source/module materialization.** InitialCompile receives the manager in
  RCX; the precompiled-descriptor request receives its object in RCX and a 16-byte-entry TArray
  result in RDX/RAX; the preprocessor constructor writes through offset `0x100` (minimum
  constructed size `0x108`); and the exact success byte is `[manager+0x388]` at the return site.
  `FAngelscriptPreprocessor::Preprocess` is RVA `0x489f410`. Its file array is at preprocessor
  `+0x58`, uses 0xc8-byte elements, and contains module shared ownership at `+0x00`, statics class
  `+0x10`, absolute/relative filenames at `+0x20/+0x30`, raw code at `+0x40` and processed code at
  `+0x68`. Final module construction at `0x489f8ec..0x489fbd1` proves module name `+0x00`, code
  array `+0x10` (0x38-byte sections), module hash `+0x20`, and section relative/absolute/code/hash
  at `+0x00/+0x10/+0x20/+0x30`. Its inline XXH64 consumes UTF-16 code bytes with seed zero and
  XORs each section result into the module hash.

  Three transient observer callsites are now exact and part of read-only image validation:
  `OnProcessChunks` CALL RVA `0x489f7e2`, bytes `e8 89 21 7a fc`, return `0x489f7e7`, direct
  callee `0x1041970`, delegate storage RVA `0x9876598`; `OnPostProcessCode` CALL RVA `0x489f8cc`,
  bytes `e8 9f 20 7a fc`, return `0x489f8d1`, the same direct callee and delegate storage RVA
  `0x98765b0`; and `ClassAnalyze` CALL RVA `0x488a1f7`, bytes `e8 64 02 00 00`, return
  `0x488a1fc`, direct callee `0x488a460`. The last call has RCX=delegate,
  RDX=`FString& generated-statics`, R8=`TSharedPtr<FAngelscriptClassDesc>`, and
  R9=`bool& bHasStatics` (a pointer to `[rbp+0x90]`, not the Boolean value). The preprocessor
  constructor proves its
  effective flag `TMap` at `+0x00` (0x50 bytes, 0x20-byte elements), custom flag strings at
  Settings `+0x28`, four effective defaults at preprocessor `+0x53..+0x56`,
  `bUseEditorScripts` at target RVA `0x9d6c4c1`, and `bUseAutomaticImportMethod` at target RVA
  `0x9d6c4e2`.

  `target_frontend_observer.cpp` implements all three pointer-neutral before/after projections.
  ClassAnalyze retains module/namespace/class, source digest, generated statics, `bHasStatics`
  and `ComposeOntoClass`. ProcessChunks and PostProcessCode hash the same ordered module/section
  source graph before the call, retain exact per-module generated declarations afterwards, and
  bind an output digest. Duplicate identities, graph mutation, order/limit drift, invalid UTF-8,
  NUL and digest substitution fail closed. Domains are exactly
  `gore-as-external-hook-graph-input-v1\0` and `gore-as-external-hook-graph-output-v1\0`, with
  little-endian u64 counts and byte lengths.
- **Complete frontend settings and target identities.** `InitialCompile` reaches the settings CDO
  through manager `+0x4d0` (`0x46847ff -> 0x4658fd0 -> CDO+0x170`). All schema booleans/enums
  are closed at Settings `+0x3a`, `+0x40`, `+0x41`, `+0x6c`, `+0x71..+0x75` and the constructor
  copies above. Both Shipping compile-time options are false. Blueprint-event specializations use
  the target `TSet` at manager `+0x478`.

  Native-super projection uses TypeUsage `GetClass(DefaultUsage)` vslot `+0x18`, then exact
  UObject/UStruct FName/Outer/SuperStruct/PropertiesSize fields `+0x18/+0x20/+0x40/+0x58`.
  Full ancestry selects Actor, ActorComponent and the five subsystem families; all other resolved
  UClasses are `other_uobject`. `CannotDeriveAngelscript` is editor-only and therefore false here.
  Static FNames are exact 8-byte rows in the TArray at RVA `0x9d6c448`, independently witnessed by
  `Bind_FName` at `0x468fa23..0x468fa3e`; FName::ToString is RVA `0x11cf640`. The first u32
  comparison index becomes the opaque key `ue5-fname-comparison-index-v1:<8 lowercase hex>`.
- **Frontend JSON and boundary mapping.** The three schema-v1 configs serialize in exact Rust
  declaration order, enforce Rust limits/order/uniqueness, zero their digest field and seal with
  the matching `gore-as-*-v1\0 || u64(json_len) || compact_json` domain. Their digests produce the
  captured config-set digest. Pointer-neutral projectors map that digest and the canonical graph
  to InitialCompile entry, either the precompiled-descriptor or constructed-preprocessor branch,
  and successful return. Descriptor arrays have 16-byte rows; the bounded reader below safely
  materializes their shared module/section content, and capabilities never enter a digest.
  Fixtures pin independent Rust/serde-compatible known answers for all config digests and the
  config-set digest.
- **Bounded target-raw frontend materializer.** `target_frontend_raw_materializer.cpp` consumes
  only a nonzero-epoch `TargetFrontendSnapshot`: a private immutable copy of explicitly typed,
  non-overlapping primary-image and data regions. Creation enforces the pinned PE image size,
  address arithmetic, a 4096-region/128-MiB aggregate bound and copies every caller buffer before
  returning. No target address is directly dereferenced or retained.

  The materializer validates 16-byte TArray/FString headers, count/capacity/backing ranges and
  strict terminated UTF-16; TSparseArray allocation bits, free counts, tail bits and exact target
  element strides for `TMap<FString,bool>`/`TSet<FString>`; and TSharedPtr object/controller
  pairing, positive bounded strong/weak counts plus an image-internal controller vtable. FName is
  decoded without target calls through the exact pool at RVA `0x9af9780`: block
  `comparison_index >> 16`, two-byte offset units, header length `>>6`, wide bit 0 and the numbered
  suffix rule. ANSI entries are restricted to unambiguous ASCII; wide entries use strict UTF-16.

  FFile is exact size `0xc8`; its TChunkedArray at `+0x50` has count `+0x60`, 113 elements per
  block and `0x90`-byte chunks. The reader closes content/comment/class/optional namespace and
  source positions, rejects active async/import-resolution state, and requires class descriptors
  to belong to the observed file. Module descriptors use 16-byte shared entries and 0x38-byte
  sections; section XXH64 over original UTF-16 and XOR module hash must match. UClass traversal
  validates image vtables, FName/Outer native package paths, SuperStruct cycles/depth and monotonic
  PropertiesSize. All outputs are pointer-neutral. The exported C++ API materializes effective
  flags, blueprint specializations, non-ASCII static-FName keys, UClass witnesses, full raw files,
  chunk/processed graphs, descriptor graphs and ClassAnalyze frames. One broad fixture exercises
  every path plus overlapping snapshots, sparse-bit drift, invalid owners and UClass cycles.

  BuildID-specific xrefs additionally prove that the ProcessChunks object at RVA `0x9876598` and
  PostProcessCode object at RVA `0x98765b0` are never bound: each has only its Broadcast LEA and
  exit destructor reference, with no Bind/Add/Remove/interior/pointer reference. Their exact
  24-byte state is pointer u64 `0`, Num/Max i32 `0/0`, CompactionThreshold i32 `2` and
  BroadcastCount i32 `0`; generic Broadcast increments/decrements the last field. The raw API
  validates both image objects at the capture epoch and
  `materialize_graph_hook_config_v1` forces both config bindings false and both capture vectors
  empty. A pointer/count/capacity, threshold or active-broadcast drift is a terminal refusal, not
  an invitation to synthesize an unsupported mutation model.
- **Production full-state shims and patch coordinator.** `production_observer_shims.asm` has one
  BuildID-pinned entry for every 9 base, 14 registration and 3 frontend site. Fixed,
  MASM-described frames preserve RFLAGS, all sixteen GPRs and XMM0..15. The 21 function/CALL
  returns replace the original return slot and correlate it through a bounded 64-deep
  `thread_local` LIFO; the common after-shim restores the slot before observer dispatch and uses
  flag-neutral LEA+RET to resume it.

  `ProductionPatchCoordinator::preflight(primary_image, session_id, observer)` is the exact
  C++ API. It allocates all rel32-near relays/trampolines, builds/registers the fourteen relocated
  AS-prolog unwind entries plus the relocated InitialCompile prolog, verifies them with
  `RtlLookupFunctionEntry`/`RtlVirtualUnwind`, and validates all 26
  source strings before `install()` can write. Install/uninstall stabilize and suspend the other
  thread set, exclude every source/generated RIP, make all pages writable before the first write,
  patch or restore as one transaction, flush, and restore protections in reverse order.
  Uninstall refuses active dispatch/return hazards, requires exact installed-or-recovery bytes,
  and retains the original protection matrix and all relay/unwind ownership until an exact,
  retryable restore succeeds.

All nine instruction spans therefore have proven frame/transfer contracts, and the three frontend
CALL witnesses have exact byte/rel32/callee contracts. The public contract now reports all nine
base bits in `statically_extractable_hook_mask` and `unresolved_hook_mask=0`. The production DLL
reports `production_installable=1`; the separately named fixture DLL reports `0`. Registry,
Build/JIT,
frontend raw/pointer-neutral projection, sparse CurrentProcess snapshots and the direct dispatcher
are connected. This flag is static implementation readiness only; it grants no authority to load
the DLL into G1R or execute the game.

## Compiler-build flag projection

The Build/JIT wire-v1 flags field has this exact allocation; bits above 7 are reserved and rejected:

| Bit | Name | BuildID 24878692 |
| ---: | --- | --- |
| 0 | `jit_info_present` | effective runtime value |
| 1 | `jit_guid_matches` | effective runtime value |
| 2 | `jit_database_cleared` | effective runtime value |
| 3 | `shipping_cache_matches` | `true` required |
| 4 | `as_reference_debugging` | `false` |
| 5 | `fork_opcode_table_201_212_present` | `true` |
| 6 | `reference_debug_opcodes_emittable` | `false` |
| 7 | `resolve_object_ptr_callback_registered` | `false` |

The pinned target's required high nibble is therefore exactly `0x20`. The donor defines
`AS_REFERENCE_DEBUGGING` from `WITH_EDITOR`; all TrackRef/UntrackRef/ValidateRef emission sites
are inside that compile-time guard, and the Shipping image has the corresponding emitters absent.
The fork opcode table still contains IDs 201 through 212, which is a table-shape fact and does not
claim that every entry is reachable. `userResolveObjectPtr` defaults null and its only donor setter
is guarded by `WITH_EDITOR && ENGINE_MAJOR_VERSION >= 5`; the pinned Shipping target therefore has
no callback and cannot normally emit opcode 204. Qualification must require opcode 204 only when
bit 7 is true, and opcodes 206 through 208 only when bit 6 is true; for this target all four counts
must prove zero despite bit 5 being true.

## Completed semantic production dispatcher

`ProductionCaptureCoordinator` is the single public-internal owner for this exact target. Its API
is `preflight(session_id, primary_image, sink)`, `install()`, `uninstall()` and
`prepare_unload()`. Preflight validates the live bridge session/image and all 26 patch plans before
the first executable write. The one direct switch consumes sites 0..8, 9..22 and 23..25 in their
typed before/after phases; nested registration returns remain LIFO-correlated. Any wrong site,
thread, phase, pointer class, container state, target value or serializer result aborts the bridge
session and makes the coordinator terminal.

`build_current_process_frontend_snapshot_v1` accepts only six closed root shapes:
configuration, settings-only configuration, module descriptors, ClassAnalyze, native UClass and
hook bindings. It uses `VirtualQuery` plus guarded same-process copies, never imports a remote
process reader, never scans words as candidate pointers and never copies the whole 0xa7e5000-byte
image. It traverses only pinned TArray/TMap/TSet/FString/FName/shared-owner/FFile/chunk/UObject/
UClass edges, merges typed sparse regions, compares every first copy against two final copies and
rejects lifetime drift before a semantic buffer is accepted.

The ClassAnalyze accessor at RVAs `0x4681ae7`/`0x4681b20` returns the exact delegate object at RVA
`0x98760a8`. The binding snapshot validates its 24-byte header, bounded 16-byte invocation entries,
zero active-broadcast count and image-owned callable targets. The descriptor-only InitialCompile
branch has no preprocessor object; its settings-only projection is backed by the constructor's
exact scalar copies at RVAs `0x4886221`, `0x4886229`, `0x4886231` and `0x4886239`
(`settings+0x3c..0x3f` to `preprocessor+0x53..0x56`). No other preprocessor state is synthesized.

All semantic records remain memory-buffered. Successful InitialCompile completion drives
RawMaterializer -> RegistrationObserver/FrontendObserver -> canonical serializers -> Bridge in
wire order and then `seal_and_detach`; a sink refusal calls `abort_and_detach` exactly once. Exact
uninstall is still required after the active return frame has resumed.

Only effective values require the separately authorized runtime observation: actual property
calls; callback/registration sequence and EAX results; encountered callable/storage/auxiliary
capabilities; resulting registry and final state; current JIT presence/GUID/clear outcome; and the
actual frontend settings, descriptors, module/source sets, ClassAnalyze binding/invocations and
outputs. ProcessChunks/PostProcessCode are statically fixed unbound, not runtime values. No such
effective value was observed or manufactured in this offline lane.

## Synthetic self-process coverage

The separately named fixture DLL exposes an internal selftest over the complete production shim
and patch surface. It proves:

- all-nine patch and exact-byte restore;
- a real 26-site self-process install/exact restore over a reserved full-size fake image,
  last-site drift refusal before writes, all-register/RFLAGS/XMM preservation, nested TLS return
  correlation and dynamic registration-trampoline unwind lookup;
- complete preflight before the first write on prolog drift;
- rollback of an injected mid-install failure;
- owner-thread-only uninstall;
- unload refusal while a transaction is installed;
- exact record order and out-of-order refusal;
- direct dispatch of all 26 site IDs across both mutually exclusive frontend-middle branches,
  with synthetic install -> buffered capture -> seal -> exact uninstall, plus order-drift ->
  single abort -> exact uninstall;
- all fourteen registration contracts, argument extraction, global order, typed result projection,
  target registration context, trace/private-ID correlation, first-seen multi-use HostStub
  derivation, all-fourteen Delta JSON variants, exact registry-support operations, all-four
  canonical FinalState JSON variants, 1:1/out-of-order FinalState sequence gates, pointer-neutral
  FinalState extraction and corruption rejection, plus target TypeUsage layout/operation
  projection and rejection of a virtual target outside the pinned image. The broad observer
  fixture additionally projects every one of the fourteen raw kinds, checks public-flag filtering,
  recursive container/template-adapter agreement, Slot-16 storage mismatch, registry-count drift
  and the exact seven final-state identities produced by its fourteen-entry trace. The frontend
  fixture covers every settings enum/boolean, UClass ancestry, non-ASCII FName identity,
  ClassAnalyze plus both graph hooks, canonical JSON/config-set digests, boundary projection and
  corruption/duplicate rejection.

The existing offline host also sends the proven property observation through the target-specific
adapter and all later synthetic records through the bridge ABI in the required order, then seals,
materializes and compares two deterministic streams. These fixture values are structural tests,
not runtime measurements or qualification evidence.
