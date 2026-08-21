# G1R compiler-profile capture helper

This directory builds a dormant Windows static library. It contains no `DllMain`, injector,
process launcher, patch, detour, or hard-coded install path. A separately authorized, version-
pinned instrumentation host must call the public API from inside the already running target.

`CaptureSession::open_pinned` fails before creating output unless the primary module and the
on-disk executable match Steam BuildID `24539464`, the exact executable byte length/SHA-256,
PE `SizeOfImage`, and CodeView RSDS GUID/age encoded in `format.hpp`. Output is created with
`CREATE_NEW`. The source and loaded module must identify the same non-reparse file. The resolved
executable directory is held open, every new output is resolved from its own non-shareable handle,
and junctions or other parent redirects into that directory are rejected. Rejected output is
deleted only by setting disposition on the exact handle that created it; a failed cleanup is
reported as recovery-required. Raw pointers are never serialized: `intern_primary_image_pointer`
accepts only addresses in the pinned primary image and records an RVA-backed token.

The writer intentionally does not know Unreal or AngelScript object layouts. The authorized
instrumentation layer must extract typed JSON using the Rust schemas. The offline Rust decoder
rejects missing phases, unknown fields, unbounded payloads, target drift, broken ordering,
unsealed streams, incomplete host-stub semantics, or a registry that cannot pass the existing
`RegistrationTraceV1` and `PostBindSnapshotV1::validate_against` checks.

No code in this directory is linked into a product target by default.
