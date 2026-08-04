# G1R AngelScript diagnostics hook

This x86-64 Windows DLL is the minimal native half of `gore as compile` diagnostics. It hooks only
the UE-AngelScript `LogAngelscriptError(asSMessageInfo*, void*)` callback, writes a bounded
line-oriented capture, filters routine per-function `Compiling ...` progress before file I/O, and
reports readiness through a complete newline-terminated token. It contains no popup hook,
diagnostic-container scan, game-path logic, or permanent installer.

The Rust launcher and DLL independently scan the same masked AOB, require exactly one raw match,
and then verify the same four-clause sparse callback-body fingerprint over a bounded `0x244`-byte
span. The fingerprint proves the first callback argument and all five `asSMessageInfo` field
offsets while masking only branch and local-stack displacements. Both sides require the AMD64
machine type plus PE32+ optional-header magic, accept only the exact `.text` section, and inspect the same raw-backed
`min(VirtualSize, SizeOfRawData)` byte range; file-alignment padding and mapped zero-fill are outside
both decisions. The known 2026-07-10 hotfix match is documented in source; no fixed RVA is used. Any
signature, structural, or hook failure causes the Rust side to terminate the diagnostic attempt and
run the ordinary compiler only after process-tree exit is confirmed.

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
`17E0AD3033C31ADD311E3C25BA63615E481C83DCF8E96E83D9B3AC088E55C01C`.

Build products (`*.o`, `ashook.dll`) are ignored/excluded; the one runtime copy is the embedded
asset under `crates/gore-as/assets/`. The build script deletes stale outputs and rejects the first
native compiler/linker command that returns a nonzero exit code.

## Third-party notice

The required MinHook 1.3.4 subset and its Hacker Disassembler Engine files are vendored under
`vendor/minhook/`. Redistribution is governed by the BSD notice in
[`vendor/minhook/LICENSE.txt`](vendor/minhook/LICENSE.txt), which must accompany source and binary
distributions. The root release `THIRD_PARTY_LICENSES.md` reproduces the same notices.
