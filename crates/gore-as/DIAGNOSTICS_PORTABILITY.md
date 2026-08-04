# AngelScript diagnostics portability matrix

Last verified offline: **2026-07-13**.

`gore as compile` does not select the diagnostic callback by executable hash, release number or a
fixed address. The Rust preflight and injected helper independently scan the raw-backed intersection
of the AMD64 PE32+ `.text` section for one masked callback entry signature, require exactly one raw
match, and verify the same sparse `asSMessageInfo` callback-body fingerprint before hooking it.

## Archived executable matrix

The following local, read-only fixtures were checked by
`tests/diagnostics_portability_test.rs`. The hashes are fixture provenance, not an allowlist.

| Archive | Bytes | SHA-256 | Raw matches | Observed RVA | Callback shape |
| --- | ---: | --- | ---: | ---: | --- |
| 1.0.0 | 171,437,056 | `740abfa9fbaae95beb5378c472ef4454df66205c140c3574eb5ba3695be53c55` | 1 | `0x467e760` | verified |
| 1.0.1 | 171,482,112 | `77f3d48ccde47756a6fa94b4b031f0ad58e2b57dcba93451415a5ed1af03f4ab` | 1 | `0x467ea50` | verified |
| 1.0.2 | 171,627,008 | `d9f45c72e624f6e27032379a7c3e51454562fd58a7eb9ac9cdaf6574c398afa9` | 1 | `0x467e200` | verified |
| 1.0.3 Hotfix 1 | 171,698,176 | `f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5` | 1 | `0x467f5b0` | verified |
| 1.0.3 Hotfix 2 | 171,704,320 | `b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d` | 1 | `0x467fcd0` | verified |

The last two rows are the first two audited generations in `gore-generation`'s now three-row
registry, and the test reads their length and digest from that table rather than restating them, so
this file is the only place they are written twice. The three older archives predate the table and
carry their own seals. Generation 24340829 is not present in this callback archive matrix and has no
archived regression evidence here; it does not inherit a callback qualification from either earlier
row. At runtime it still receives the ordinary signature/shape probe: a verified match may enable
capture, while a missing or invalid match safely uses the normal compiler fallback.

All five callback RVAs differ. They are reported for auditing and regression only; neither scanner
uses them for lookup.

Run the matrix from the repository root:

```powershell
cargo test -p gore-as --test diagnostics_portability_test -- --nocapture
```

The test discovers the local archive at `work/reversing/binaries` or accepts an explicit root via
`GORE_AS_RELEASE_MATRIX_DIR`. The large proprietary executables are not distributed with the crate,
so a checkout without fixtures skips the matrix. An explicitly configured or locally present matrix
must contain all five exact files.

## Capability and fallback contract

- One raw AOB match plus the verified callback structure makes diagnostic capture available. Captured
  messages are rendered as conventional `file:line:column: severity: message` compiler output.
- Zero matches, multiple matches, a structural mismatch, unsupported PE layout, missing helper or a
  confirmed injection failure is an availability failure, not a compiler result. After confirming
  that the first process is gone and deleting any partial development cache, compilation runs again
  through the unchanged normal game generator.
- If process exit cannot be confirmed, no fallback process is started and recovery artifacts are
  preserved. This avoids two concurrent generators and unsafe cleanup.
- The helper repeats the signature, section and callback-shape checks in the mapped image before
  enabling MinHook. A successful offline preflight alone never authorizes an unverified address.

The synthetic regression suite covers zero/duplicate matches, shape mismatch, PE and section bounds,
helper non-materialization on mismatch, clean fallback, injection failure and unconfirmed-exit
handling. The archived matrix itself never launches the game, injects a DLL or modifies a fixture.

## Future and non-Steam executables

They are deliberately **not claimed compatible in advance**. They receive the same capability probe:
AMD64 PE32+, exact `.text` scan domain, one signature and the verified callback layout. Passing that
probe establishes compatibility with this diagnostic callback contract only. Failure safely disables
enhanced diagnostics and uses the normal compiler path; it must not be bypassed with a remembered RVA.
The same probe rule applies to registered generation 24340829. Adding a qualified retained fixture to
this matrix would establish repeatable offline regression evidence; its absence does not pre-decide the
runtime probe result.
