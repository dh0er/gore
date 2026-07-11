# AngelScript diagnostics hook

`gore-as-diagnostics-hook.dll` is embedded into `gore` and materialized only for the lifetime of a
compiler run. It is never installed into Gothic, UE4SS, or the game directory.

- SHA-256: `9A408F1DD2BFD95B9235D261C94B4FF60EB4C562BA5D1D69B398BE9499E9F153`
- Source/build recipe: `crates/gore-as/native/diagnostics-hook/`
- Vendored MinHook/HDE source and required BSD notice:
  `crates/gore-as/native/diagnostics-hook/vendor/minhook/LICENSE.txt`
- Release notice: root `THIRD_PARTY_LICENSES.md`
- Deterministic Windows rebuild and asset refresh:
  `powershell -File crates/gore-as/native/diagnostics-hook/build.ps1 -UpdateEmbeddedAsset`

The Rust launcher and DLL independently require exactly one masked AOB match for the per-message
`LogAngelscriptError` callback in the same raw-backed byte range of an AMD64 image's exact `.text`
section. A mismatch, ambiguous match, incomplete capture, or unsupported image disables acceptance
of diagnostic output; safe infrastructure failures fall back to the ordinary game compiler.
