# AngelScript diagnostics portability matrix

Last verified offline: **2026-08-28**.

`gore as compile` does not select diagnostics by executable hash, release number, or a fixed
address. The Rust preflight and injected helper independently scan the raw-backed intersection of
the AMD64 PE32+ `.text` section for two masked entry signatures: the ordinary AngelScript callback
and `FAngelscriptManager::ScriptCompileError`, which receives ClassGenerator diagnostics directly.
Each must have exactly one raw match and its matching sparse structure fingerprint before either
hook is enabled.

## Archived executable matrix

The following local, read-only fixtures were checked by
`tests/diagnostics_portability_test.rs`. The hashes are fixture provenance, not an allowlist.

| Archive | Bytes | SHA-256 | Callback RVA | Manager RVA | Both shapes |
| --- | ---: | --- | ---: | ---: | --- |
| 1.0.0 | 171,437,056 | `740abfa9fbaae95beb5378c472ef4454df66205c140c3574eb5ba3695be53c55` | `0x467e760` | `0x46825f0` | verified |
| 1.0.1 | 171,482,112 | `77f3d48ccde47756a6fa94b4b031f0ad58e2b57dcba93451415a5ed1af03f4ab` | `0x467ea50` | `0x46828e0` | verified |
| 1.0.2 | 171,627,008 | `d9f45c72e624f6e27032379a7c3e51454562fd58a7eb9ac9cdaf6574c398afa9` | `0x467e200` | `0x4682090` | verified |
| 1.0.3 Hotfix 1 | 171,698,176 | `f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5` | `0x467f5b0` | `0x4683440` | verified |
| 1.0.3 Hotfix 2 | 171,704,320 | `b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d` | `0x467fcd0` | `0x4683b60` | verified |
| 1.0.4 / BuildID 24340829 | 171,787,776 | `ab2c8d9e286a437bc5343748faf40959a77e9dc7c542ff9361f1ffaeca5c811c` | `0x46861d0` | `0x468a060` | verified |
| 1.0.4a / BuildID 24539464 | 171,784,704 | `c71c04dd86e11e3e94483ea02c26c612b6243c147f6d83973233b3c8ddc5de25` | `0x4685ff0` | `0x4689e80` | verified |
| 1.0.5 / BuildID 24878692 | 171,792,384 | `824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0` | `0x4685fb0` | `0x4689e40` | verified |

The two 1.0.3 rows and the BuildID-24340829/24878692 rows read their length and digest from
`gore-generation`; the remaining four archives carry explicit fixture seals. The hashes establish
fixture provenance only and are not an address allowlist.

All eight callback RVAs and all eight manager RVAs differ. They are reported for auditing and
regression only; neither scanner uses them for lookup.

Run the matrix from the repository root:

```powershell
cargo test -p gore-as --test diagnostics_portability_test -- --nocapture
```

The test discovers the local archive at `work/reversing/binaries` or accepts an explicit root via
`GORE_AS_RELEASE_MATRIX_DIR`. The large proprietary executables are not distributed with the crate,
so a checkout without fixtures skips the matrix. An explicitly configured or locally present matrix
must contain all eight exact files.

## Capability and fallback contract

- One raw match and a verified structure for each of the two boundaries makes diagnostic capture
  available. Captured messages are rendered as conventional
  `file:line:column: severity: message` compiler output.
- Zero matches, multiple matches, a structural mismatch, unsupported PE layout, missing helper or a
  confirmed injection failure is an availability failure, not a compiler result. After confirming
  that the first process is gone and deleting any partial development cache, compilation runs again
  through the unchanged normal game generator.
- If process exit cannot be confirmed, no fallback process is started and recovery artifacts are
  preserved. This avoids two concurrent generators and unsafe cleanup.
- The helper repeats both signature, section, and shape checks in the mapped image before enabling
  either MinHook detour. A successful offline preflight alone never authorizes an unverified address.

The synthetic regression suite covers zero/duplicate matches, shape mismatch, PE and section bounds,
helper non-materialization on mismatch, clean fallback, injection failure and unconfirmed-exit
handling. The archived matrix itself never launches the game, injects a DLL or modifies a fixture.

## Future and non-Steam executables

They are deliberately **not claimed compatible in advance**. They receive the same capability probe:
AMD64 PE32+, exact `.text` scan domain, both unique signatures, and both verified layouts. Passing
that probe establishes compatibility with this diagnostic-capture contract only. Failure safely
disables enhanced diagnostics and uses the normal compiler path; it must not be bypassed with a
remembered RVA. Adding a qualified retained fixture to this matrix establishes repeatable offline
regression evidence; its absence does not pre-decide a future runtime probe result.
