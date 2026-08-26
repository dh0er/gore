# Offline differential qualification V1

The artifact, comparison, and promotion APIs never launch a process: they observe already-produced
artifacts, compare embedded and standalone results, and publish a create-new qualified profile
package. The separate `StandaloneQualificationHarnessV1` is the productive offline backend: it
launches only the exactly pinned standalone sidecar through the qualification-only protocol. No
API in this lane launches the game.

## Accepted semantic authority

`CompilerProbeBackendV1` cannot submit a semantic digest. An accepted observation must contain the
complete cache bytes and, for `Invoke`, a typed `CanonicalInvokeReturnV1`. The runner always calls
`observe_whole_cache_semantics_v1`; rejected observations may contain structured diagnostics only.
The following fail closed:

- missing, extra, truncated, trailing, or misaligned cache data;
- unresolved or ambiguous pointer/ID references and legacy `ByteCodeReferences`;
- accepted diagnostics containing an error or rejected diagnostics lacking an error;
- compile-only returns, missing accepted invoke returns, or rejected returns;
- a semantic-observer contract other than `gore.as.whole-cache-semantic-observer/v1`.

The observer covers every module and function, bytecode, locals/stack/object-variable data,
UFunction/property/class/behaviour/import/module metadata, globals and initializer functions, all
seven tail tables, and the optional canonical return. Raw pointers and IDs are replaced only by
complete resolved semantic identities. The whole-cache digest and every per-module digest have
separate fixed SHA-256 domains. Module order remains significant; tail maps are sorted by their
complete resolved identities.

## Sealed corpus and concrete gates

`full_qualification_corpus_v1()` returns 27 ordered cases under
`gore.as.full-differential-qualification/v2`. The current exact corpus/source seal is:

`6220f1671c42a771e4895e55ded4bad49247140f00a7d77ba7857fe4c6fe2761`

The corpus includes positive and negative syntax, overloads/defaults, templates/validators,
containers, namespaces/imports, metadata, class generation, scalar and structured invocation,
fork bytecode and reference lifetimes, globals/classes/behaviours/properties/accessors, all tails,
preprocessor/import closure, the exact Diego dialog-authoring example, Unicode `FName`, UTF-8 `FString` factory behavior, frontend hooks,
editor/release discovery, graph change/delete/add, located info/warning/error diagnostics, and
unsupported try/catch.

Coverage labels are routing metadata only. `qualified=true` additionally requires observed
witnesses:

- every accepted case cache contains every corpus source module for that case;
- reachable fork opcodes are actually present, while `DestructScript`, reference-debug opcodes,
  `SaveReturnValue`, and target-disabled `ResolveObjectPtr` are exactly absent. The concrete source
  reaches `CopyScript` through assignment of a non-POD script value containing `TArray<int32>`,
  reaches `CpyVtoR1` and `FreeNullV8` through a conditional object-handle return, and retains
  independent construction, null comparison, switch-exhaustiveness, reference/out/inout, and
  exception paths for `FinConstruct`, `CmpPtrNull`, and `ThrowException`;
- the Shipping capture flags prove `AS_REFERENCE_DEBUGGING=false` and
  `resolve_object_ptr_callback_registered=false` for the unresolved-property policy;
- the model probe has nonzero class, behaviour, property, global, initializer, and T1–T7 counts;
- `FName` returns canonical `true`; UTF-8 `FString` returns `Grüße_日本`, uses a string global
  reference, and emits no canonical `STR` opcode;
- the same-run frontend case recreates the authority capture's non-ASCII `FName` spellings and
  proves their pairwise comparison-equivalence partition; Unreal's numeric comparison indices are
  process-local allocation tokens and are normalized only after that semantic check;
- located warning, overload-context info, and located error coverage is aggregated across the
  appropriate accepted/rejected cases;
- all three frontend hook capture sets, generated declarations, and editor/release discovery
  sets are nonempty;
- unresolved runtime identities and legacy bytecode-reference mutations are refused.

Some cases are marked `requires_captured_profile`. They remain mandatory. A host without the exact
captured registrations must fail the complete V1 suite rather than omit or relabel those cases.

## Graph transition authority

For `CompileGraphTransition`, `OfflineCapturedProbeOutputV1::accepted_graph_transition` requires raw
baseline and final cache bytes. The generator observes both; a backend cannot supply module lists
or semantic digests. The sealed artifact entry retains the baseline blob seal, its whole-cache
witness, and baseline/final per-module semantic identities.

Acceptance proves that changed modules retain their map identity and change semantics, deleted
modules occur only in the baseline, added modules occur only in the final cache, every untouched
base module is bytecode-semantically identical, and neither cache has an unlisted add/delete/change.
This supports real full-graph caches containing the entire product base; corpus sections describe
only the transition overlay, not the complete module universe.

## Artifact generation, promotion, and publication

`capture_and_seal_offline_qualification_artifacts_v1` consumes one ordered
`OfflineQualificationCaptureBackendV1` run. It computes all cache witnesses locally, seals both
accepted final caches and graph baselines, and immediately reloads through
`OfflineCompilerProbeArtifactBackendV1::load`.

`promote_generated_offline_qualification_artifacts_v1` accepts only generated in-memory artifact
authorities. It derives expected results from the embedded run and requires exact parity for cache
semantics, diagnostics, returns, frontend traces, graph witnesses, and build flags. It returns no
promotion token when any unexplained difference remains.

`promote_unqualified_profile_package_v1` consumes that token plus an exactly pinned unqualified
profile package and publishes a new, no-clobber, typed-reloaded qualified package. The durable
qualification files are:

- `embedded-qualification-artifacts.json`;
- `standalone-qualification-artifacts.json`;
- `qualification-promotion-receipt.json`.

`QualifiedProfilePromotionReceiptV1::{embedded_artifacts,standalone_artifacts}` binds each backend
kind, suite/corpus identity, the exact unqualified source-profile SHA-256 and target tuple, the
actual standalone executable/protocol identity (standalone artifacts only), canonical and raw
manifest SHA-256, every final/baseline cache seal, the cache-seal aggregate, and the
supplemental-witness aggregate. Capture rechecks this authority at every case boundary. Embedded
and standalone artifacts must identify the same source profile and target; promotion derives the
sidecar identity from the standalone artifact instead of accepting a caller label. Raw caches may live in an external immutable archive, but
their complete seals cannot be removed from the promoted receipt. `reload_qualified_profile_package_v1`
recomputes these authorities before accepting the package.

Release-input packaging must require those three filenames and a successful
`reload_qualified_profile_package_v1`; the read-only
`gore-as-qualified-profile-verifier <absolute-profile-root>` binary is the stable subprocess seam
for non-Rust packaging. It emits a small JSON success record containing the typed profile SHA-256.
Copying only `compiler-profile.json` and its runtime payloads is not an authority-preserving
promotion check.

## Productive standalone execution boundary

`StandaloneQualificationHarnessV1` is the productive, offline implementation of
`OfflineQualificationCaptureBackendV1`. It owns `full_qualification_corpus_v1()` and rejects any
caller case which is not byte-for-byte the corresponding sealed row. Construction strictly reloads
the fixed-name unqualified materializer package, binds its corpus payload seal to the executable
27-case corpus, verifies Shipping/Binds against the profile, and verifies the opened sidecar against
an externally authorized final executable identity. The external identity is the one unavoidable
pre-promotion input: an unqualified package cannot authenticate the binary which is about to qualify
it. It must come from the signed sidecar release/catalog authority, never the package directory.

The additive `qualify --request` protocol v3 is not a product compile protocol and is therefore not
recorded as `QualifiedSidecarIdentityV1.request_version`; that remains FullGraph v2/response v1.
Only the explicit qualification command accepts v3 and a typed `qualified=false` profile. Requests
carry source/corpus/case/phase/invoke identity but no witness fields. Successful responses bind the
SHA-256 of the complete request and return raw cache bytes plus same-process diagnostics, effective
build flags, frontend hook hits/generated declarations/editor-release discovery, and an
optional typed invoke result. Unknown request fields, including attempted caller witnesses, fail.

The graph case intentionally uses two process executions, one for the raw baseline and one for the
raw final graph. This is not claimed as a single native process. The replacement invariant is
stronger than a caller-supplied transition label: both executions reopen the same SHA-256-pinned
sidecar, restage the same sealed profile/Shipping/Binds inputs, bind the same suite/corpus/case, use
distinct sealed phases, echo the complete request digest, and return raw caches. The Rust generator
then derives the transition solely by whole-cache observation. Neither process accepts module lists
or transition witnesses from the caller.

Primitive zero-argument invoke remains inside an instruction-aligned closed VM whitelist. The three
mandatory host-value cases use qualification-only donor adapters whose activation is pinned to the
captured declaration, call convention, layout, type-operation flags, hidden-metadata state, string
factory, and FName comparison keys. `TArray<int32>` returns a two-element zero-initialized array via
donor `SetNum` semantics, non-ASCII `FName` compares captured identities, and `FString` roundtrips
UTF-8 through the donor UTF-16 layout. Bytecode may call only those exact reflected adapters,
allocate/free only qualified types, and address only strings owned by the qualification factory.
Product compilation continues to use inert registry stubs; a missing or mismatched registration is
terminally unavailable. Likewise, a captured-profile case stops if its exact registration or
frontend-hook capture is absent. The embedded-game execution adapter remains separately
authorization-gated.
