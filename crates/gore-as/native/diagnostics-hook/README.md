# G1R AngelScript diagnostics hook

This x86-64 Windows DLL is the minimal native half of `gore as compile` diagnostics. It hooks the
UE-AngelScript `LogAngelscriptError(asSMessageInfo*, void*)` callback and the structured
`FAngelscriptManager::ScriptCompileError(const FString&, const FDiagnostic&)` insertion boundary.
The second path is required for ClassGenerator/Unreal-reflection errors that never pass through the
ordinary AngelScript callback. It writes one bounded line-oriented capture, filters routine
per-function `Compiling ...` progress before file I/O, and reports readiness through a complete
newline-terminated token. It contains no popup hook, diagnostic-container scan, game-path logic, or
permanent installer.

The Rust launcher and DLL independently scan the same two masked AOBs, require exactly one raw match
for each, and verify the same sparse body fingerprints. The `0x244` callback span proves all five
`asSMessageInfo` fields; the `0xb0` manager span proves the Windows/x64 arguments plus every
`FString`/`FDiagnostic` field consumed by the detour. Only irrelevant branch, call, and local-stack
operands are masked. Both sides require the AMD64 machine type plus PE32+ optional-header magic,
accept only the exact `.text` section, and inspect the same raw-backed
`min(VirtualSize, SizeOfRawData)` byte range; file-alignment padding and mapped zero-fill are outside
both decisions. The known RVAs are documented in source but never used for lookup. Any signature,
structural, or hook failure causes the Rust side to terminate the diagnostic attempt and run the
ordinary compiler only after process-tree exit is confirmed.

The read-only four-release regression results and the exact future/non-Steam capability contract are
documented in [`../../DIAGNOSTICS_PORTABILITY.md`](../../DIAGNOSTICS_PORTABILITY.md).

Capture content reserves room below the 8 MiB limit for a newline-terminated truncation marker. The
Rust side treats either that marker or a file reaching the hard cap as incomplete and never accepts
an otherwise successful cache when compiler errors may have been omitted.

## Build and verify

The build requires a 64-bit MinGW GCC/G++ toolchain. `CC` and `CXX` may override the defaults.
GNU ld's timestamp insertion is disabled, so identical inputs/toolchain produce identical bytes.

```powershell
powershell -File crates/gore-as/native/diagnostics-hook/build.ps1 -UpdateEmbeddedAsset
Get-FileHash crates/gore-as/assets/gore-as-diagnostics-hook.dll -Algorithm SHA256
```

Expected embedded SHA-256:
`3D9852ED4A077C0B987A290FD1B349A92AF394FEEF297A33165464B2E4C2E39D`.

Build products (`*.o`, `ashook.dll`) are ignored/excluded; the one runtime copy is the embedded
asset under `crates/gore-as/assets/`. The build script deletes stale outputs and rejects the first
native compiler/linker command that returns a nonzero exit code.

## Third-party notice

The required MinHook 1.3.4 subset and its Hacker Disassembler Engine files are vendored under
`vendor/minhook/`. Redistribution is governed by the BSD notice in
[`vendor/minhook/LICENSE.txt`](vendor/minhook/LICENSE.txt), which must accompany source and binary
distributions. The root release `THIRD_PARTY_LICENSES.md` reproduces the same notices.
