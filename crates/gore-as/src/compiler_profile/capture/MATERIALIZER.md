# Offline compiler-profile materializer V1

This lane turns one already sealed native capture plus ten separately pinned static support
payloads into a complete, self-sealed **unqualified** `CompilerProfileV1` directory. It neither
loads the capture DLL nor starts, injects into, or attaches to a process. It does not run an
oracle, qualify a compiler, or create a deployable compiler package.

## Command

Build and run from the repository root on Windows x64:

```powershell
cargo build -p gore-as --bin gore-as-profile-materializer
cargo run -p gore-as --bin gore-as-profile-materializer -- `
  C:\absolute\input\capture.capture `
  C:\absolute\support\static-support.json `
  C:\absolute\support\payloads `
  C:\absolute\output\new-profile-directory
```

All four paths must be absolute and normalized (no `.` or `..`). The output parent must exist;
the final output directory must not exist. Success prints one JSON summary to stdout. Failure is
nonzero and never overwrites an existing output directory. If publication fails after the new
directory was created, it is deliberately left as an unusable partial directory for inspection;
retry with another new path.

## Inputs and trust boundary

1. `capture.capture` is the original sealed V1 stream. The CLI opens it without following a
   reparse point, holds the single-link file handle, bounds its size, and passes its bytes to
   `decode_capture_v1`. The decoder's strict, fully validated `DecodedCaptureV1` projection is
   the only capture-derived value accepted by the inner materializer. The native
   `*.wire-summary.json` is audit output and is **not** an input.
   Its Build/JIT flags are validated as an exact target invariant: bit 4
   `as_reference_debugging=false`, bit 5 `fork_opcode_table_201_212_present=true`, bit 6
   `reference_debug_opcodes_emittable=false`, and bit 7
   `resolve_object_ptr_callback_registered=false`; bits above 7 are reserved. Thus the compiler
   high nibble must be exactly `0x20`. Table presence does not make opcode 204 or 206..208
   reachable, and qualification must prove those target-disabled opcode counts remain zero.
2. `static-support.json` has schema
   `gore.as.unqualified-profile-static-support`, version `1`. It pins the exact BuildID
   `24539464` Windows/x64/Shipping target, executable identity and CodeView identity, the Binds
   measurements, static format-version strings, and byte length plus SHA-256 for every payload.
3. The support root contains the ten fixed-name payloads consumed by the manifest:

   - `reflected-type-graph.bin`
   - `opcode-table.bin`
   - `operand-schema.bin`
   - `codegen-probe-corpus.json`
   - `expected-probe-results.json`
   - `serializer-schema.bin`
   - `reference-table-order.bin`
   - `normalized-oracle-corpus.bin`
   - `diagnostic-parity.json`
   - `semantic-parity.json`

The static manifest JSON maps these through the `payloads` object using the snake-case keys
`reflected_type_graph`, `opcode_table`, `operand_schema`, `codegen_probe_corpus`,
`expected_probe_results`, `serializer_schema`, `reference_table_order`,
`normalized_oracle_corpus`, `diagnostic_parity`, and `semantic_parity`. Each value is:

```json
{ "byte_len": 123, "sha256": "64 hexadecimal characters" }
```

The complete manifest shape is:

```json
{
  "schema": "gore.as.unqualified-profile-static-support",
  "schema_version": 1,
  "target": {
    "steam_app_id": 1297900,
    "steam_build_id": 24539464,
    "depot_id": 1297901,
    "depot_manifest_gid": 1585071322101748861,
    "platform": "windows",
    "architecture": "x86_64",
    "build_configuration": "shipping"
  },
  "oracle": {
    "executable": { "byte_len": 171784704, "sha256": "c71c04dd86e11e3e94483ea02c26c612b6243c147f6d83973233b3c8ddc5de25", "steam_content_sha1": "<40 hex>" },
    "binds_cache": { "byte_len": 1, "sha256": "<64 hex>", "steam_content_sha1": "<40 hex>" },
    "shipping_cache": { "byte_len": 1, "sha256": "<64 hex>", "steam_content_sha1": "<40 hex>" },
    "depot_manifest": { "byte_len": 1, "sha256": "<64 hex>" },
    "pe_codeview": { "guid": "cf0b83bd-e023-061b-2100-0f0fccf871d2", "age": 1 }
  },
  "binds": {
    "wire_schema_version": 1,
    "struct_count": 1,
    "class_count": 1,
    "method_count": 1,
    "struct_property_count": 1,
    "class_property_count": 1,
    "canonical_database_sha256": "<64 hex>"
  },
  "unreal_metadata_schema_version": 1,
  "opcode_table_version": "<nonempty pinned version>",
  "cache_format_version": 1,
  "required_probe_suite_version": "<nonempty pinned version>",
  "payloads": {
    "reflected_type_graph": { "byte_len": 1, "sha256": "<64 hex>" },
    "opcode_table": { "byte_len": 1, "sha256": "<64 hex>" },
    "operand_schema": { "byte_len": 1, "sha256": "<64 hex>" },
    "codegen_probe_corpus": { "byte_len": 1, "sha256": "<64 hex>" },
    "expected_probe_results": { "byte_len": 1, "sha256": "<64 hex>" },
    "serializer_schema": { "byte_len": 1, "sha256": "<64 hex>" },
    "reference_table_order": { "byte_len": 1, "sha256": "<64 hex>" },
    "normalized_oracle_corpus": { "byte_len": 1, "sha256": "<64 hex>" },
    "diagnostic_parity": { "byte_len": 1, "sha256": "<64 hex>" },
    "semantic_parity": { "byte_len": 1, "sha256": "<64 hex>" }
  }
}
```

The `1` values are shape examples, not accepted production measurements; replace every count,
length, digest, and version with the pinned evidence for the support set. Unknown fields are
rejected.

Every input file and support directory is opened no-follow and retained by handle while its
bytes are checked. Support files must be regular, non-reparse, single-link files. A manifest seal
mismatch aborts before the output root is created.

## Outputs

The new directory contains 18 CREATE_NEW/no-follow, single-link files:

- six typed decoder projections: `engine-properties.json`, `registration-trace.json`,
  `post-bind-snapshot.json`, `preprocessor-config.json`, `class-generator-config.json`, and
  `compiler-options.json`;
- the ten byte-identical static support payloads listed above;
- `compiler-profile.json`, whose canonical `profile_sha256` covers the complete
  `CompilerProfileV1` and whose `qualification.qualified` is forcibly `false`;
- `materialization-receipt.json`, whose canonical seal binds the capture stream SHA-256, the
  exact static-support manifest SHA-256, the profile SHA-256, and seals for the other 17 files.

The two parity-named support files are opaque, seal-pinned inputs at this stage. Their presence is
not qualification evidence and never changes the forced `qualified=false` state.

Before reporting success, the materializer rereads every output from the still-retained file
handles, reparses the profile and receipt into typed V1 values, validates the typed engine and
frontend projections, rechecks all 16 manifest blob seals, and proves that the normal
`CompilerProfileV1::from_json` product parser rejects the package as `NotQualified`.
`reload_unqualified_profile_package_v1` repeats the read-only typed/seal/receipt verification for
an existing output directory. It cannot set qualification.

The resulting directory is therefore a qualification input, not a compiler package. A later,
separately authorized qualification lane must produce new parity evidence and a newly canonical
`qualified=true` manifest. Until then the existing product resolver cannot consume this profile.

## Synthetic and corruption gates

The focused Windows tests build a complete synthetic sealed capture, decode it through
`decode_capture_v1`, materialize all 18 outputs with synthetic sealed support, require the typed
reload to succeed, and require the qualified product parser to reject it. The same test mutates a
materialized projection and requires typed reload failure. A second test mutates a support
payload before publication and verifies that no output directory is created, then verifies that
an existing output directory and sentinel file are never clobbered.

Run the grouped gates with:

```powershell
cargo test -p gore-as compiler_profile::capture::materialize --lib
cargo test -p gore-as compiler_profile::manifest --lib
cargo check -p gore-as --bin gore-as-profile-materializer
```
